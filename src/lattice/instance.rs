use talk::unicast::Message as UnicastMessage;

pub(crate) trait Instance: Clone + Eq + UnicastMessage {}

impl<I> Instance for I where I: Clone + Eq + UnicastMessage {}
