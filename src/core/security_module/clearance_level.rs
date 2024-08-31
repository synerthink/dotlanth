#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ClearanceLevel {
    Employee,
    Manager,
    Executive,
    Board,
}
