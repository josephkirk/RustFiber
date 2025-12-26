
#[cfg(test)]
mod tests {
    use corosensei::stack::DefaultStack;
    use corosensei::Coroutine;

    #[test]
    fn test_stack_api() {
        let size = 1024 * 1024;
        let mut stack = DefaultStack::new(size).unwrap();
        let _coro = Coroutine::with_stack(&mut stack, |_, _: ()| {
            // ...
        });
    }
}
