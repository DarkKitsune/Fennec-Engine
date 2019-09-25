use crate::error::FennecError;
use std::mem::replace;
use std::mem::MaybeUninit;

pub struct IteratorOk<TOk> {
    items: Vec<MaybeUninit<TOk>>,
    current: usize,
}

impl<TOk> Iterator for IteratorOk<TOk> {
    type Item = TOk;
    fn next(&mut self) -> Option<<Self as Iterator>::Item> {
        if self.current >= self.items.len() {
            None
        } else {
            let garbage = MaybeUninit::uninit();
            let ret = replace(&mut self.items[self.current], garbage);
            self.current += 1;
            Some(unsafe { ret.assume_init() })
        }
    }
}

pub trait IteratorResults<TOk> {
    fn handle_results(self) -> Result<IteratorOk<TOk>, FennecError>;
}

impl<TIterator, TOk> IteratorResults<TOk> for TIterator
where
    TIterator: Iterator<Item = Result<TOk, FennecError>>,
{
    fn handle_results(self) -> Result<IteratorOk<TOk>, FennecError> {
        let mut items = Vec::new();
        for item in self {
            items.push(MaybeUninit::new(item?));
        }
        Ok(IteratorOk {
            items: items,
            current: 0usize,
        })
    }
}
