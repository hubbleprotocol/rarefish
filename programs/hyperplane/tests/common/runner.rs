use solana_program_test::ProgramTest;

use super::types::TestContext;
use crate::common::fixtures::ProgramDependency;
use solana_program_test::anchor_processor;

pub fn program(dependencies: &[ProgramDependency]) -> ProgramTest {
    let program_test = ProgramTest::new("hyperplane", hyperplane::ID, None);

    dependencies
        .iter()
        .for_each(|_dep| unimplemented!("No dependency supported yet."));
    program_test
}

pub async fn start(test: ProgramTest) -> TestContext {
    let mut context = test.start_with_context().await;
    let rent = context.banks_client.get_rent().await.unwrap();

    TestContext { context, rent }
}
