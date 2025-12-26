use crossbeam::deque::Worker;

#[test]
fn test_deque_ordering_lifo_local() {
    // Verify that Worker::new_lifo() provides LIFO behavior for local pop
    // (push to bottom, pop from bottom)
    let w = Worker::<i32>::new_lifo();
    w.push(1);
    w.push(2);
    w.push(3);

    // Expect LIFO order: 3, 2, 1
    assert_eq!(w.pop(), Some(3));
    assert_eq!(w.pop(), Some(2));
    assert_eq!(w.pop(), Some(1));
    assert_eq!(w.pop(), None);
}

#[test]
fn test_deque_ordering_fifo_steal() {
    // Verify that Worker::new_lifo() provides FIFO behavior for thieves
    // (push to bottom, steal from top)
    let w = Worker::<i32>::new_lifo();
    w.push(1);
    w.push(2);
    w.push(3);

    let s = w.stealer();

    // Expect FIFO order: 1, 2, 3
    assert_eq!(s.steal(), crossbeam::deque::Steal::Success(1));
    assert_eq!(s.steal(), crossbeam::deque::Steal::Success(2));
    assert_eq!(s.steal(), crossbeam::deque::Steal::Success(3));
    assert_eq!(s.steal(), crossbeam::deque::Steal::Empty);
}
