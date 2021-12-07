use hound::WavWriter;
use cs140_common::buffer::Buffer;
use cs140_common::descriptor::SampleFormat::F32;
use cs140_common::padding::padding_inclusive_range;
use cs140_common::record::Recorder;
use cs140_network::encoding::{BitStore, HandlePackage};
use cs140_network::physical_test::PhysicalLayer;
use cs140_network::physical_test::PhysicalPackage;

#[tokio::main]
async fn main() {
    let mut builder = env_logger::Builder::from_default_env();
    builder.format_timestamp_millis().init();

    let data: Vec<u8> = vec![202, 202, 202, 202, 202, 202, 202, 202, 202, 202, 202, 202, 202, 202, 202, 202];
    println!("{}", data[0]);
    let data = BitStore::from_vec(data);
    println!("{}", data);
    let mut layer = PhysicalLayer::new(1, 1, 1024);
    layer.send(PhysicalPackage {
        0: data.clone()
    }).await;

    layer.send(PhysicalPackage {
        0: data.clone()
    }).await;

    let data:Vec<_> = layer.input_buffer.pop_by_ref(48001,|x|{
        (x.iter().cloned().collect(),0)
    }).await;

    let mut descriptor = layer.output_descriptor;
    descriptor.sample_format = F32;
    let writer = WavWriter::create("w.wav", descriptor.into()).unwrap();
    let recorder = Recorder::new(writer, 1 * descriptor.sample_rate as usize);
    let recorder = recorder.record_from_slice(&data);
    drop(recorder);

    tokio::time::sleep(std::time::Duration::from_secs(10)).await;


    layer.receive().await;

}