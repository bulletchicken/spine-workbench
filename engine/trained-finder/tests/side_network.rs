use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use shapes::errors::ErrorCode;
use trained_finder::{
    grid::Grid,
    side::{
        self, decode,
        table::{self, Packet},
        weights::WeightSetup,
    },
};

fn header() -> Value {
    let mut tensors = Vec::new();
    let mut offset = 0;
    for layer in table::layers() {
        for (suffix, shape) in [
            (
                "weight",
                vec![layer.outputs, layer.inputs, layer.kernel, layer.kernel],
            ),
            ("bias", vec![layer.outputs]),
        ] {
            let count = shape.iter().product::<usize>();
            tensors.push(json!({"name":format!("{}.{}",layer.name,suffix),"shape":shape,"offset":offset,"count":count}));
            offset += count * 4;
        }
    }
    assert_eq!(offset, table::PAYLOAD_BYTES);
    json!({"format":"side-finder-weights","version":1,"model":"side_lumbar_heatmap_v1","checkpointSha256":table::CHECKPOINT,"batchNormEps":0.00001,"batchNormFolded":true,"inputShape":[1,512,384],"outputShape":[20,128,96],"bytes":offset,"payloadSha256":"0".repeat(64),"tensors":tensors})
}

fn packet(header: &Value, payload: &[u8]) -> Vec<u8> {
    let text = serde_json::to_vec(header).unwrap();
    let mut bytes = b"SWSIDE01".to_vec();
    bytes.extend((text.len() as u32).to_le_bytes());
    bytes.extend(text);
    bytes.extend(payload);
    bytes
}

#[test]
fn the_packet_schema_spans_and_tensor_set_are_strict() {
    let good = header();
    let payload = vec![0; table::PAYLOAD_BYTES];
    let bytes = packet(&good, &payload);
    let parsed = Packet::read(&bytes).unwrap();
    assert_eq!(parsed.payload.len(), 10_203_888);
    let changes: Vec<(&str, Value)> = vec![
        ("/format", json!("other")),
        ("/version", json!(2)),
        ("/model", json!("other")),
        ("/checkpointSha256", json!("0".repeat(64))),
        ("/batchNormEps", json!(0.001)),
        ("/batchNormFolded", json!(false)),
        ("/inputShape", json!([1, 384, 512])),
        ("/outputShape", json!([20, 96, 128])),
        ("/bytes", json!(4)),
        ("/payloadSha256", json!("z".repeat(64))),
        ("/tensors/0/shape", json!([24, 1, 3, 2])),
        ("/tensors/0/count", json!(0)),
        ("/tensors/0/offset", json!(4)),
        ("/tensors/1/offset", json!(0)),
        ("/tensors/0/name", json!("unused.weight")),
        ("/tensors/1/name", json!("e1.0.weight")),
        ("/tensors/0/count", json!(usize::MAX)),
        ("/tensors/0/shape", json!([usize::MAX, usize::MAX])),
    ];
    for (pointer, value) in changes {
        let mut changed = good.clone();
        *changed.pointer_mut(pointer).unwrap() = value;
        assert_eq!(
            Packet::read(&packet(&changed, &payload))
                .err()
                .unwrap()
                .code,
            ErrorCode::EngineLoadFailed,
            "{pointer}"
        );
    }
    let mut changed = good.clone();
    changed["unknown"] = json!(1);
    assert!(Packet::read(&packet(&changed, &payload)).is_err());
    changed = good.clone();
    changed["tensors"][0]["unknown"] = json!(1);
    assert!(Packet::read(&packet(&changed, &payload)).is_err());
    changed = good.clone();
    changed["tensors"].as_array_mut().unwrap().pop();
    assert!(Packet::read(&packet(&changed, &payload)).is_err());
    for n in [0, 7, 8, 11, 12, bytes.len() - 1] {
        assert!(Packet::read(&bytes[..n]).is_err());
    }
    let mut changed = bytes.clone();
    changed[0] = 0;
    assert!(Packet::read(&changed).is_err());
    changed = bytes.clone();
    changed.extend([0; 4]);
    assert!(Packet::read(&changed).is_err());
    changed = bytes.clone();
    changed[8..12].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(Packet::read(&changed).is_err());
    // Duplicate JSON keys are refused by the typed schema too.
    let text = serde_json::to_string(&good)
        .unwrap()
        .replacen('{', "{\"version\":1,", 1);
    changed = b"SWSIDE01".to_vec();
    changed.extend((text.len() as u32).to_le_bytes());
    changed.extend(text.as_bytes());
    changed.extend(&payload);
    assert!(Packet::read(&changed).is_err());
}

#[test]
fn float_decoding_is_deferred_bounded_and_refuses_nonfinite_values() {
    let good = header();
    let mut payload = vec![0; table::PAYLOAD_BYTES];
    for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        let end = payload.len();
        payload[end - 4..].copy_from_slice(&value.to_le_bytes());
        let bytes = packet(&good, &payload);
        let mut setup = WeightSetup::new(&bytes);
        let error = loop {
            match setup.advance() {
                Ok(false) => {}
                Ok(true) => panic!("nonfinite payload accepted"),
                Err(error) => break error,
            }
        };
        assert_eq!(error.code, ErrorCode::EngineLoadFailed);
    }
    let bytes = packet(&good, &vec![0; table::PAYLOAD_BYTES]);
    let mut setup = WeightSetup::new(&bytes);
    assert_eq!(setup.units_done(), 0);
    assert!(!setup.is_done());
    while !setup.advance().unwrap() {}
    assert_eq!(setup.units_done(), WeightSetup::units());
    let count = setup.units_done();
    assert!(setup.advance().unwrap());
    assert_eq!(setup.units_done(), count);
    assert!(setup.finish().is_ok());
}

#[test]
fn availability_is_presence_and_the_present_payload_hash_is_checked() {
    assert_eq!(
        side::available(),
        trained_finder::side::weights::packet().is_some()
    );
    if let Some(bytes) = trained_finder::side::weights::packet() {
        // A damaged present packet is still compiled availability.
        if let Ok(parsed) = Packet::read(bytes) {
            assert_eq!(
                Sha256::digest(parsed.payload)
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>(),
                parsed.payload_sha256
            );
        }
    } else {
        assert_eq!(
            WeightSetup::compiled().err().unwrap().code,
            ErrorCode::SideFinderUnavailable
        );
    }
}

#[test]
fn centroids_keep_first_peaks_and_truncate_the_window_at_all_edges() {
    let flat = Grid::zeros(20, 9, 9).unwrap();
    assert_eq!(decode::channel(&flat, 0).unwrap(), [7.5, 7.5]);
    for (x, y) in [
        (0, 0),
        (4, 0),
        (8, 0),
        (0, 4),
        (8, 4),
        (0, 8),
        (4, 8),
        (8, 8),
        (4, 4),
    ] {
        let mut maps = Grid::from_values(20, 9, 9, vec![-1000.0; 20 * 81]).unwrap();
        maps.row_mut(0, y)[x] = 1000.0;
        assert_eq!(
            decode::channel(&maps, 0).unwrap(),
            [(x as f32 + 0.5) * 4.0 - 0.5, (y as f32 + 0.5) * 4.0 - 0.5]
        );
    }
    let mut tied = Grid::from_values(20, 9, 9, vec![-1000.0; 20 * 81]).unwrap();
    tied.row_mut(0, 0)[0] = 1000.0;
    tied.row_mut(0, 8)[8] = 1000.0;
    assert_eq!(decode::channel(&tied, 0).unwrap(), [1.5, 1.5]);
    let mut maps = Grid::zeros(20, 15, 17).unwrap();
    for c in 0..20 {
        let x = c % 10 + 3;
        let y = c / 10 + 4;
        maps.row_mut(c, y)[x] = f32::MAX;
        assert_eq!(
            decode::channel(&maps, c).unwrap(),
            [x as f32 * 4.0 + 1.5, y as f32 * 4.0 + 1.5]
        );
    }
    for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        maps.row_mut(0, 0)[0] = value;
        assert_eq!(
            decode::channel(&maps, 0).unwrap_err().code,
            ErrorCode::EngineOutOfSync
        );
    }
    assert!(decode::channel(&flat, 20).is_err());
    assert!(decode::channel(&Grid::zeros(19, 9, 9).unwrap(), 0).is_err());
}
