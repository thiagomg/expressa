#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg, // -
    Not, // nao
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    // arithmetic
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    // comparison
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    // logical
    And, // e
    Or,  // ou
    // membership
    Contem,
    // note: se_falhar is its own Expr variant, not BinaryOp
}
