use super::*;
use std::{
    cell::Cell,
    fmt,
};

pub type MirMeta = Repr;
pub type MirNode<T> = Node<T, MirMeta>;

// TODO: Keep track of scope, perhaps?
#[derive(Copy, Clone, Debug)]
pub struct LocalId(usize);

#[derive(Clone, Debug, PartialEq)]
pub enum Const {
    Nat(u64),
    Int(i64),
    Num(f64),
    Char(char),
    Bool(bool),
    Str(Intern<String>),
    Tuple(Vec<Self>),
    List(Vec<Self>),
    Sum(usize, Box<Self>),
}

impl Const {
    pub fn bool(&self) -> bool { if let Const::Bool(x) = self { *x } else { panic!("{:?}", self) } }
    pub fn nat(&self) -> u64 { if let Const::Nat(x) = self { *x } else { panic!("{:?}", self) } }
    pub fn int(&self) -> i64 { if let Const::Int(x) = self { *x } else { panic!("{:?}", self) } }
    pub fn num(&self) -> f64 { if let Const::Num(x) = self { *x } else { panic!("{:?}", self) } }
    pub fn char(&self) -> char { if let Const::Char(c) = self { *c } else { panic!("{:?}", self) } }
    pub fn list(&self) -> Vec<Self> { if let Const::List(x) = self { x.clone() } else { panic!("{:?}", self) } }
}

#[derive(Clone, Debug)]
pub enum Intrinsic {
    MakeList(Repr),

    NotBool,

    NegNat,
    AddNat,
    SubNat,
    MulNat,
    DivNat,
    RemNat,
    EqNat,
    NotEqNat,
    LessNat,
    MoreNat,
    LessEqNat,
    MoreEqNat,

    NegInt,
    AddInt,
    SubInt,
    MulInt,
    DivInt,
    EqInt,
    NotEqInt,
    LessInt,
    MoreInt,
    LessEqInt,
    MoreEqInt,

    NegNum,
    AddNum,
    SubNum,
    MulNum,
    DivNum,
    EqNum,
    NotEqNum,
    LessNum,
    MoreNum,
    LessEqNum,
    MoreEqNum,

    EqChar,
    NotEqChar,

    Join(Repr),
}

#[derive(Clone, Debug)]
pub enum Pat {
    Wildcard,
    Const(Const), // Expression is evaluated and then compared
    Single(MirNode<Binding>),
    Add(MirNode<Binding>, u64),
    Tuple(Vec<MirNode<Binding>>),
    ListExact(Vec<MirNode<Binding>>),
    ListFront(Vec<MirNode<Binding>>, Option<MirNode<Binding>>),
    Variant(usize, MirNode<Binding>),
}

#[derive(Clone, Debug)]
pub struct Binding {
    pub pat: Pat,
    pub name: Option<Ident>,
}

impl Binding {
    pub fn is_refutable(&self) -> bool {
        match &self.pat {
            Pat::Wildcard => false,
            Pat::Const(c) => match c {
                Const::Tuple(fields) if fields.is_empty() => false,
                _ => true,
            },
            Pat::Single(inner) => inner.is_refutable(),
            Pat::Add(lhs, rhs) => *rhs > 0 || lhs.is_refutable(),
            Pat::Tuple(fields) => fields
                .iter()
                .any(|field| field.is_refutable()),
            Pat::ListExact(_) => true,
            Pat::ListFront(items, tail) => items.len() > 0 || tail.as_ref().map_or(false, |tail| tail.is_refutable()),
            Pat::Variant(_, _) => true, // TODO: Check number of variants
        }
    }

    fn visit_bindings(&self, mut bind: &mut impl FnMut(Ident)) {
        self.name.map(&mut bind);
        match &self.pat {
            Pat::Wildcard => {},
            Pat::Const(_) => {},
            Pat::Single(inner) => inner.visit_bindings(bind),
            Pat::Add(lhs, _) => lhs.visit_bindings(bind),
            Pat::Tuple(fields) => fields
                .iter()
                .for_each(|field| field.visit_bindings(bind)),
            Pat::ListExact(items) => items
                .iter()
                .for_each(|item| item.visit_bindings(bind)),
            Pat::ListFront(items, tail) => {
                items
                    .iter()
                    .for_each(|item| item.visit_bindings(bind));
                tail.as_ref().map(|tail| tail.visit_bindings(bind));
            },
            Pat::Variant(_, inner) => inner.visit_bindings(bind),
        }
    }

    pub fn binding_names(&self) -> Vec<Ident> {
        let mut names = Vec::new();
        self.visit_bindings(&mut |name| names.push(name));
        names
    }

    pub fn binds(&self) -> bool {
        let mut binds = false;
        self.visit_bindings(&mut |_| binds = true);
        binds
    }
}

#[derive(Copy, Clone, Debug)]
pub struct GlobalFlags {
    /// Determines whether a global reference may be inlined. By default this is `true`, but inlining is not permitted
    /// for recursive definitions.
    pub can_inline: bool,
}

impl Default for GlobalFlags {
    fn default() -> Self {
        Self {
            can_inline: true,
        }
    }
}

#[derive(Clone, Debug)]
pub enum Expr {
    Const(Const),
    Local(Ident),
    Global(ProcId, Cell<GlobalFlags>),

    Intrinsic(Intrinsic, Vec<MirNode<Self>>),
    Match(MirNode<Self>, Vec<(MirNode<Binding>, MirNode<Self>)>),

    // (captures, arg, body)
    Func(Vec<Ident>, Ident, MirNode<Self>),
    Apply(MirNode<Self>, MirNode<Self>),

    Tuple(Vec<MirNode<Self>>),
    Access(MirNode<Self>, usize),
    List(Vec<MirNode<Self>>),

    Variant(usize, MirNode<Self>),
    AccessVariant(MirNode<Self>, usize), // Unsafely assume the value is a specific variant

    Debug(MirNode<Self>),
}

impl Expr {
    fn required_locals_inner(&self, stack: &mut Vec<Ident>, required: &mut Vec<Ident>) {
        match self {
            Expr::Const(_) => {},
            Expr::Local(local) => {
                if !stack.contains(local) {
                    required.push(*local);
                }
            },
            Expr::Global(_, _) => {},
            Expr::Intrinsic(_, args) => args
                .iter()
                .for_each(|arg| arg.required_locals_inner(stack, required)),
            Expr::Match(pred, arms) => {
                pred.required_locals_inner(stack, required);
                for (arm, body) in arms {
                    let old_stack = stack.len();
                    stack.append(&mut arm.binding_names());

                    body.required_locals_inner(stack, required);

                    stack.truncate(old_stack);
                }
            },
            Expr::Func(captures, arg, body) => {
                for capture in captures {
                    if !stack.contains(capture) {
                        required.push(*capture);
                    }
                }

                let old_stack = stack.len();
                stack.extend(captures.iter().copied());
                stack.push(*arg);

                body.required_locals_inner(stack, required);

                stack.truncate(old_stack);
            },
            Expr::Apply(f, arg) => {
                f.required_locals_inner(stack, required);
                arg.required_locals_inner(stack, required);
            },
            Expr::Tuple(fields) => fields
                .iter()
                .for_each(|field| field.required_locals_inner(stack, required)),
            Expr::List(items) => items
                .iter()
                .for_each(|item| item.required_locals_inner(stack, required)),
            Expr::Access(tuple, _) => tuple.required_locals_inner(stack, required),
            Expr::Variant(_, inner) => inner.required_locals_inner(stack, required),
            Expr::AccessVariant(inner, _) => inner.required_locals_inner(stack, required),
            Expr::Debug(inner) => inner.required_locals_inner(stack, required),
        }
    }

    pub fn required_locals(&self, already_has: impl IntoIterator<Item = Ident>) -> Vec<Ident> {
        let mut required = Vec::new();
        self.required_locals_inner(&mut already_has.into_iter().collect(), &mut required);
        required
    }

    pub fn print(&self) -> impl fmt::Display + '_ {
        struct DisplayBinding<'a>(&'a Binding, usize);

        impl<'a> fmt::Display for DisplayBinding<'a> {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                if let Some(name) = self.0.name {
                    write!(f, "{}{}", if name.starts_with(|c: char| c.is_alphabetic()) { "" } else { "$" }, name)?;
                    if let Pat::Wildcard = &self.0.pat {
                        return Ok(());
                    } else {
                        write!(f, " ~ ")?;
                    }
                }
                match &self.0.pat {
                    Pat::Wildcard => write!(f, "_"),
                    Pat::Const(c) => write!(f, "const {:?}", c),
                    Pat::Single(inner) => write!(f, "{}", DisplayBinding(inner, self.1)),
                    Pat::Variant(variant, inner) => write!(f, "${} {}", variant, DisplayBinding(inner, self.1)),
                    Pat::ListExact(items) => write!(f, "[{}]", items.iter().map(|i| format!("{},", DisplayBinding(i, self.1 + 1))).collect::<Vec<_>>().join(" ")),
                    Pat::ListFront(items, tail) => write!(
                        f,
                        "[{} .. {}]",
                        items.iter().map(|i| format!("{},", DisplayBinding(i, self.1 + 1))).collect::<Vec<_>>().join(" "),
                        tail.as_ref().map(|tail| format!("{}", DisplayBinding(tail, self.1))).unwrap_or_default(),
                    ),
                    Pat::Tuple(fields) => write!(f, "({})", fields.iter().map(|f| format!("{},", DisplayBinding(f, self.1 + 1))).collect::<Vec<_>>().join(" ")),
                    // _ => write!(f, "<PAT>"),
                    pat => todo!("{:?}", pat),
                }
            }
        }

        struct DisplayExpr<'a>(&'a Expr, usize);

        impl<'a> fmt::Display for DisplayExpr<'a> {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                use Intrinsic::*;
                match self.0 {
                    Expr::Local(local) => write!(f, "{}{}", if local.starts_with(|c: char| c.is_alphabetic()) { "" } else { "$" }, local),
                    Expr::Global(global, _) => write!(f, "global {:?}", global),
                    Expr::Const(c) => write!(f, "const {:?}", c),
                    Expr::Func(_, arg, body) => write!(f, "fn {}{} => {}", if arg.starts_with(|c: char| c.is_alphabetic()) { "" } else { "$" }, arg, DisplayExpr(body, self.1)),
                    Expr::Apply(func, arg) => write!(f, "({})({})", DisplayExpr(func, self.1), DisplayExpr(arg, self.1)),
                    Expr::Variant(variant, inner) => write!(f, "${} {}", variant, DisplayExpr(inner, self.1)),
                    Expr::Tuple(fields) => write!(f, "({})", fields.iter().map(|f| format!("{},", DisplayExpr(f, self.1 + 1))).collect::<Vec<_>>().join(" ")),
                    Expr::List(items) => write!(f, "[{}]", items.iter().map(|i| format!("{},", DisplayExpr(i, self.1 + 1))).collect::<Vec<_>>().join(" ")),
                    Expr::Intrinsic(NotBool, args) => write!(f, "!{}", DisplayExpr(&args[0], self.1)),
                    Expr::Intrinsic(NegNat | NegInt | NegNum, args) => write!(f, "-{}", DisplayExpr(&args[0], self.1)),
                    Expr::Intrinsic(AddNat | AddInt | AddNum, args) => write!(f, "{} + {}", DisplayExpr(&args[0], self.1), DisplayExpr(&args[1], self.1)),
                    Expr::Intrinsic(SubNat | SubInt | SubNum, args) => write!(f, "{} - {}", DisplayExpr(&args[0], self.1), DisplayExpr(&args[1], self.1)),
                    Expr::Intrinsic(MulNat | MulInt | MulNum, args) => write!(f, "{} * {}", DisplayExpr(&args[0], self.1), DisplayExpr(&args[1], self.1)),
                    Expr::Intrinsic(DivNat | DivInt | DivNum, args) => write!(f, "{} / {}", DisplayExpr(&args[0], self.1), DisplayExpr(&args[1], self.1)),
                    Expr::Intrinsic(RemNat, args) => write!(f, "{} % {}", DisplayExpr(&args[0], self.1), DisplayExpr(&args[1], self.1)),
                    Expr::Intrinsic(EqNat | EqInt | EqNum | EqChar, args) => write!(f, "{} = {}", DisplayExpr(&args[0], self.1), DisplayExpr(&args[1], self.1)),
                    Expr::Intrinsic(LessNat | LessInt | LessNum, args) => write!(f, "{} < {}", DisplayExpr(&args[0], self.1), DisplayExpr(&args[1], self.1)),
                    Expr::Intrinsic(MoreNat | MoreInt | MoreNum, args) => write!(f, "{} > {}", DisplayExpr(&args[0], self.1), DisplayExpr(&args[1], self.1)),
                    Expr::Intrinsic(MoreEqNat | MoreEqInt | MoreEqNum, args) => write!(f, "{} >= {}", DisplayExpr(&args[0], self.1), DisplayExpr(&args[1], self.1)),
                    Expr::Intrinsic(LessEqNat | LessEqInt | LessEqNum, args) => write!(f, "{} <= {}", DisplayExpr(&args[0], self.1), DisplayExpr(&args[1], self.1)),
                    Expr::Intrinsic(Join(_), args) => write!(f, "{} ++ {}", DisplayExpr(&args[0], self.1), DisplayExpr(&args[1], self.1)),
                    Expr::Match(pred, arms) => {
                        write!(f, "match {} in", DisplayExpr(pred, self.1))?;
                        for (arm, body) in arms {
                            write!(f, "\n{}| {} => {}", "    ".repeat(self.1 + 1), DisplayBinding(arm, self.1 + 1), DisplayExpr(body, self.1 + 1))?;
                        }
                        Ok(())
                    },
                    Expr::Debug(inner) => write!(f, "?{}", DisplayExpr(inner, self.1)),
                    // _ => write!(f, "<TODO>"),
                    expr => todo!("{:?}", expr),
                }
            }
        }

        DisplayExpr(self, 0)
    }
}
