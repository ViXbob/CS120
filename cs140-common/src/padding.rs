use rand::distributions::{Distribution, Standard};
use rand::Rng;

pub fn padding<T>() -> impl Iterator<Item = T> where Standard: Distribution<T>{
    let mut rng = rand::thread_rng();
    return std::iter::repeat(0).map(move |_|rng.gen());
}

pub fn padding_range<T>(start: T, end:T) -> impl Iterator<Item = T> where T : rand::distributions::uniform::SampleUniform + std::cmp::PartialOrd + Copy + Clone{
    let mut rng = rand::thread_rng();
    let range = std::ops::Range{
        start,
        end,
    };
    return std::iter::repeat(0).map(move |_|rng.gen_range(range.clone()));
}

#[cfg(test)]
mod test{
    use crate::padding::padding;

    #[test]
    fn test_padding(){
        let p = padding();
        let vec:Vec<u8> = p.take(100000).collect();
        assert!(vec.len() > 0)
    }
}