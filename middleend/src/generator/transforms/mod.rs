mod expression;
mod function;
mod block_item;
mod declaration;
mod program;
mod statement;
mod block;

pub use expression::*;
pub use function::*;
pub use block_item::*;
pub use declaration::*;
pub use program::*;
pub use statement::*;
pub use block::*;

use super::TacGenerator;

pub struct GeneratorTransforms<'a> {
    generator: &'a mut TacGenerator,
}

impl<'a> GeneratorTransforms<'a> {
    pub fn new(generator: &'a mut TacGenerator) -> Self {
        Self { generator }
    }
}
