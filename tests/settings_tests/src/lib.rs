#[cfg(test)]
mod tests {
    use googletest::assert_that;
    use googletest::matchers::eq;

    #[googletest::gtest]
    fn right() {
        let a = 1;
        let b = 2;
        let sum = a + b;
        assert_that!(sum, eq(3));
    }

    #[googletest::gtest]
    fn wrong() {
        let a = 1;
        let b = 2;
        let sum = a + b;
        assert_that!(sum, eq(4));
    }
}