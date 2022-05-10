pub mod ownership;
pub mod utils;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        near_sdk::testing_env!(near_sdk::test_utils::VMContextBuilder::new()
            .attached_deposit(near_sdk::ONE_NEAR)
            .build());

        // near_sdk::testing_env!();
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
