#[macro_use(shards)]
extern crate reed_solomon_erasure;

use reed_solomon_erasure::galois_8::{ReedSolomon};
// or use the following for Galois 2^16 backend
// use reed_solomon_erasure::galois_16::ReedSolomon;

pub fn make_redundancy(data: Vec<u8>, padding: usize, redundancy_ratio: f32) -> (Vec<Vec<u8>>, ReedSolomon) {
    let mut data: Vec<Vec<u8>> = data.chunks(padding - 1).enumerate().map(|(index, chunk)| {
        let mut shard = Vec::with_capacity(padding);
        shard.extend(chunk.into_iter());
        shard.resize(padding - 1, 0);
        shard
    }).collect();
    let data_shard_count = data.len();
    let redundancy_shard_count = (redundancy_ratio * data.len() as f32).floor() as usize;
    data.extend(std::iter::repeat_with(|| std::iter::repeat(0).take(padding - 1).collect()).take(redundancy_shard_count));
    let r = ReedSolomon::new(data_shard_count, redundancy_shard_count).unwrap();
    assert!(data_shard_count + redundancy_shard_count < 256);
    r.encode(&mut data).unwrap();
    for (index, vec) in data.iter_mut().enumerate() {
        vec.insert(0, index as u8);
    }
    return (data, r);
}

pub fn erase_redundancy(mut data: Vec<Option<Vec<u8>>>, r: ReedSolomon, length: usize) -> anyhow::Result<Vec<u8>> {
    for vec in data.iter_mut() {
        if let Some(vec) = vec {
            vec.remove(0);
        }
    }
    r.reconstruct(&mut data)?;
    let mut result: Vec<u8> = data.into_iter().map(|x|x.unwrap()).flatten().take(length).collect();
    Ok(result)
}

#[test]
fn main() {
    let data: Vec<_> = (0..=255).cycle().take(10000).collect();
    let (master_copy, r) = make_redundancy(data.clone(), 1000, 1.0);

    // Make a copy and transform it into option shards arrangement
    // for feeding into reconstruct_shards
    let mut shards: Vec<_> = master_copy.iter().cloned().map(Some).collect();

    // We can remove up to 2 shards, which may be data or parity shards
    shards[0] = None;
    shards[4] = None;
    shards[3] = None;
    shards[7] = None;
    let formatted = erase_redundancy(shards, r, 10000).unwrap();
    assert_eq!(data,formatted)
    // Try to reconstruct missing shards
    //r.reconstruct(&mut shards).unwrap();

    // Convert back to normal shard arrangement
    //let result: Vec<_> = shards.into_iter().filter_map(|x| x).collect();

    // assert!(r.verify(&result).unwrap());
    // assert_eq!(master_copy, result);
}
