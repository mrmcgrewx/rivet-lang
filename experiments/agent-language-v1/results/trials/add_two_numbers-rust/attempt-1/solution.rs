pub fn solve(
    first: &Option<Box<Node>>,
    second: &Option<Box<Node>>,
) -> Option<Box<Node>> {
    let mut first_node = first.as_deref();
    let mut second_node = second.as_deref();
    let mut carry = 0;
    let mut result = None;
    let mut tail = &mut result;

    while first_node.is_some() || second_node.is_some() || carry != 0 {
        let mut sum = carry;
        if let Some(node) = first_node {
            sum += node.value;
            first_node = node.next.as_deref();
        }
        if let Some(node) = second_node {
            sum += node.value;
            second_node = node.next.as_deref();
        }

        carry = sum / 10;
        tail = &mut tail
            .insert(Box::new(Node {
                value: sum % 10,
                next: None,
            }))
            .next;
    }

    result
}
