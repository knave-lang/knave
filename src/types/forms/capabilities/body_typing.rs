use super::{AnnotationLowering, ChildTyping, ExpectedChildTyping, LocalScope};

pub(in crate::types) trait BodyTyping:
    ChildTyping + ExpectedChildTyping + AnnotationLowering + LocalScope
{
}

impl<C> BodyTyping for C where C: ChildTyping + ExpectedChildTyping + AnnotationLowering + LocalScope
{}
