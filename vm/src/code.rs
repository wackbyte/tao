use super::*;
use std::io::Write;

#[derive(Clone, Debug)]
pub enum Instr {
    Error(&'static str),
    Nop,
    Break,

    Call(isize),
    Ret,
    MakeFunc(isize, usize), // Make a function using the relative offset and by capturing the last N items on the stack
    ApplyFunc,

    MakeList(usize), // T * N => [T]
    IndexList(usize), // Nth field of list/tuple
    SkipList(usize), // (N..) fields of list/tuple
    LenList,
    JoinList,

    MakeSum(usize),
    IndexSum(usize),
    VariantSum,

    Jump(isize),
    IfNot,

    Imm(Value),
    Pop(usize),
    Replace,
    Dup, // Duplicate value on top of stack
    PushLocal,
    PopLocal(usize), // Don't push to stack
    GetLocal(usize), // Duplicate value in locals position (len - 1 - N) and put on stack

    NotBool, // Bool -> Bool
    AndBool, // Bool -> Bool -> Bool
    EqBool, // Bool -> Bool -> Bool

    NegInt, // Int -> Int
    AddInt, // Int -> Int -> Int
    SubInt, // Int -> Int -> Int
    MulInt, // Int -> Int -> Int
    DivInt, // Int -> Int -> Num
    RemInt, // Int -> Int -> Int
    EqInt, // Int -> Int -> Bool
    LessInt, // Int -> Int -> Bool
    MoreInt, // Int -> Int -> Bool
    LessEqInt, // Int -> Int -> Bool
    MoreEqInt, // Int -> Int -> Bool

    NegNum, // Num -> Num
    AddNum, // Num -> Num -> Num
    SubNum, // Num -> Num -> Num
    MulNum, // Num -> Num -> Num
    DivNum, // Num -> Num -> Num
    EqNum, // Num -> Num -> Bool
    LessNum, // Num -> Num -> Bool
    MoreNum, // Num -> Num -> Bool
    LessEqNum, // Num -> Num -> Bool
    MoreEqNum, // Num -> Num -> Bool

    EqChar, // Char -> Char -> Bool
}

impl Instr {
    pub fn bool(x: bool) -> Self {
        Self::Imm(Value::Bool(x))
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct Addr(pub usize);

impl Addr {
    pub fn incr(self) -> Self { Self(self.0 + 1) }
    pub fn jump(self, rel: isize) -> Self { Self((self.0 as isize + rel) as usize) }
    pub fn jump_to(self, other: Self) -> isize { other.0 as isize - self.0 as isize }
}

#[derive(Default, Debug)]
pub struct Program {
    instrs: Vec<Instr>,
    pub entry: Addr,
    debug: Vec<(Addr, String)>,
}

impl Program {
    pub fn debug(&mut self, msg: impl ToString) {
        self.debug.push((self.next_addr(), msg.to_string()));
    }

    pub fn next_addr(&self) -> Addr { Addr(self.instrs.len()) }

    pub fn instr(&self, ip: Addr) -> Instr {
        self.instrs
            .get(ip.0)
            .cloned()
            .unwrap_or(Instr::Error("out of bounds instruction"))
    }

    pub fn push(&mut self, instr: Instr) -> Addr {
        let addr = self.next_addr();
        self.instrs.push(instr);
        addr
    }

    pub fn fixup(&mut self, addr: Addr, tgt: Addr, make_instr: impl FnOnce(isize) -> Instr) {
        self.instrs[addr.0] = make_instr(addr.jump_to(tgt));
    }

    pub fn write(&self, mut writer: impl Write) {
        let mut debug = self.debug.iter().peekable();
        for addr in (0..self.instrs.len()).map(Addr) {
            while debug.peek().map_or(false, |(a, _)| *a == addr) {
                writeln!(writer, " ...  | <--------- {}", debug.next().unwrap().1).unwrap();
            }

            let instr = self.instr(addr);

            let stack_diff = match instr {
                Instr::Error(_) | Instr::Nop | Instr::Break => 0,
                Instr::Imm(_) => 1,
                Instr::Pop(n) => -(n as isize),
                Instr::Replace => -1,
                Instr::Call(_) => 0,
                Instr::Ret => 0,
                Instr::MakeFunc(_, n) => -(n as isize),
                Instr::ApplyFunc => 0, // Turns input stack item into output stack item
                Instr::MakeList(n) => -(n as isize) + 1,
                Instr::IndexList(_) => 0,
                Instr::SkipList(_) => 0,
                Instr::LenList => 0,
                Instr::JoinList => -1,
                Instr::MakeSum(_) => 0,
                Instr::IndexSum(_) => 0,
                Instr::VariantSum => 0,
                Instr::Dup => 1,
                Instr::Jump(_) => 0,
                Instr::IfNot => -1,
                Instr::PushLocal => -1,
                Instr::PopLocal(_) => 0,
                Instr::GetLocal(_) => 1,
                Instr::NotBool
                | Instr::NegInt
                | Instr::NegNum => 0,
                Instr::EqBool
                | Instr::AndBool
                | Instr::AddInt
                | Instr::SubInt
                | Instr::MulInt
                | Instr::DivInt
                | Instr::RemInt
                | Instr::EqInt
                | Instr::LessInt
                | Instr::MoreInt
                | Instr::LessEqInt
                | Instr::MoreEqInt
                | Instr::AddNum
                | Instr::SubNum
                | Instr::MulNum
                | Instr::DivNum
                | Instr::EqNum
                | Instr::LessNum
                | Instr::MoreNum
                | Instr::LessEqNum
                | Instr::MoreEqNum
                | Instr::EqChar => -1,
            };

            let instr_display = match instr {
                Instr::Error(msg) => format!("error \"{}\"", msg),
                Instr::Nop => format!("nop"),
                Instr::Break => format!("break"),
                Instr::Imm(x) => format!("imm `{}`", x),
                Instr::Pop(n) => format!("pop {}", n),
                Instr::Replace => format!("replace"),
                Instr::Call(x) => format!("call {:+} (0x{:03X})", x, addr.jump(x).0),
                Instr::Ret => format!("ret"),
                Instr::MakeFunc(i, n) => format!("func.make {:+} (0x{:03X}) {}", i, addr.jump(i).0, n),
                Instr::ApplyFunc => format!("func.apply"),
                Instr::MakeList(n) => format!("list.make {}", n),
                Instr::IndexList(i) => format!("list.index #{}", i),
                Instr::SkipList(i) => format!("list.skip #{}", i),
                Instr::LenList => format!("list.len"),
                Instr::JoinList => format!("list.join"),
                Instr::MakeSum(i) => format!("sum.make #{}", i),
                Instr::IndexSum(i) => format!("sum.index #{}", i),
                Instr::VariantSum => format!("sum.variant"),
                Instr::Dup => format!("dup"),
                Instr::Jump(x) => format!("jump {:+} (0x{:03X})", x, addr.jump(x).0),
                Instr::IfNot => format!("if_not"),
                Instr::PushLocal => format!("local.push"),
                Instr::PopLocal(n) => format!("local.pop {}", n),
                Instr::GetLocal(x) => format!("local.get +{}", x),
                Instr::NotBool => format!("bool.not"),
                Instr::AndBool => format!("bool.and"),
                Instr::EqBool => format!("bool.eq"),
                Instr::NegInt => format!("int.neg"),
                Instr::AddInt => format!("int.add"),
                Instr::SubInt => format!("int.sub"),
                Instr::MulInt => format!("int.mul"),
                Instr::DivInt => format!("int.div"),
                Instr::RemInt => format!("int.rem"),
                Instr::EqInt => format!("int.eq"),
                Instr::LessInt => format!("int.less"),
                Instr::MoreInt => format!("int.more"),
                Instr::LessEqInt => format!("int.less_eq"),
                Instr::MoreEqInt => format!("int.more_eq"),
                Instr::NegNum => format!("num.neg"),
                Instr::AddNum => format!("num.add"),
                Instr::SubNum => format!("num.sub"),
                Instr::MulNum => format!("num.mul"),
                Instr::DivNum => format!("num.div"),
                Instr::EqNum => format!("num.eq"),
                Instr::LessNum => format!("num.less"),
                Instr::MoreNum => format!("num.more"),
                Instr::LessEqNum => format!("num.less_eq"),
                Instr::MoreEqNum => format!("num.more_eq"),
                Instr::EqChar => format!("char.eq"),
            };

            writeln!(writer, "0x{:03X} | {:>+3} | {}", addr.0, stack_diff, instr_display).unwrap();
        }

        writeln!(writer, "{} instructions in total.", self.instrs.len()).unwrap();
    }
}
