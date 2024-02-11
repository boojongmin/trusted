#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ZeroCurveCode{
    Undefined = 0,
    KRWGOV = 1,
    KRWIRS = 2,
    KRWOIS = 3,
    KRWCRS = 4,
    USDGOV = 5,
    USDIRS = 6,
    USDOIS = 7,
}