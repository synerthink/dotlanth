use assert_cmd::Command;

#[test]
fn test_execution_engine() {
    let mut cmd = Command::cargo_bin("dotvm").unwrap();
    cmd.assert().success().stderr(predicates::str::contains(
        "SimpleAdditionContract result: 8",
    ));
}

#[test]
fn test_protocol_manager() {
    let mut cmd = Command::cargo_bin("dotvm").unwrap();
    cmd.assert().success().stderr(predicates::str::contains(
        "ExpenseApprovalProtocol approval required: true",
    ));
}
