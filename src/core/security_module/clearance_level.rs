/// Represents different levels of security clearance in the system.
///
/// Clearance levels are ordered from lowest to highest:
/// Employee < Manager < Executive < Board
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ClearanceLevel {
    Employee,
    Manager,
    Executive,
    Board,
}
