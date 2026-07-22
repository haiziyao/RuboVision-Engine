use async_trait::async_trait;
use rubo_engine::{FuncResult, Function, FunctionCall, FunctionError};

macro_rules! define_result_sample {
    ($type:ident, $id:literal, $output:expr) => {
        #[rubo_engine::function(id = $id)]
        #[derive(Default)]
        pub struct $type;

        impl $type {
            fn output() -> serde_json::Value {
                $output
            }
        }

        #[async_trait]
        impl Function for $type {
            async fn call(&self, _call: FunctionCall<'_>) -> Result<FuncResult, FunctionError> {
                Ok(FuncResult::new(Self::output()))
            }
        }
    };
}

define_result_sample!(
    ColorResultSample,
    "sample_color_result",
    serde_json::json!({
        "text": "color_detect sample: blue",
        "name": "blue",
        "value": "blue",
        "ratio": 0.95,
        "sample": true
    })
);

define_result_sample!(
    QrResultSample,
    "sample_qr_result",
    serde_json::json!({
        "text": "qr_detect sample: RUBO-QR-SAMPLE",
        "value": "RUBO-QR-SAMPLE",
        "sample": true
    })
);

define_result_sample!(
    ConcentricRingResultSample,
    "sample_concentric_ring_result",
    serde_json::json!({
        "text": "concentric_ring sample: CROSS,0,1,-12,8,92",
        "value": "CROSS,0,1,-12,8,92",
        "found": true,
        "dx": -12,
        "dy": 8,
        "score": 92,
        "sample": true
    })
);

define_result_sample!(
    BlackRingResultSample,
    "sample_black_ring_result",
    serde_json::json!({
        "text": "black_ring_detect sample: RING,1,-5,11,88",
        "value": "RING,1,-5,11,88",
        "found": true,
        "dx": -5,
        "dy": 11,
        "score": 88,
        "sample": true
    })
);

define_result_sample!(
    LetterResultSample,
    "sample_letter_result",
    serde_json::json!({
        "text": "letter_detect sample: A",
        "value": "A",
        "holes": 1,
        "sample": true
    })
);

define_result_sample!(
    ColorBlockResultSample,
    "sample_color_block_result",
    serde_json::json!({
        "text": "color_block_detect sample: BLOCK,blue,1,-20,15",
        "value": "BLOCK,blue,1,-20,15",
        "found": true,
        "color": "blue",
        "color_output": "blue",
        "center_x": 300,
        "center_y": 255,
        "dx": -20,
        "dy": 15,
        "area": 25000.0,
        "bbox": { "x": 210, "y": 165, "width": 180, "height": 180 },
        "sample": true
    })
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn result_samples_match_uart_values() {
        let samples = [
            (ColorResultSample::output(), "blue"),
            (QrResultSample::output(), "RUBO-QR-SAMPLE"),
            (ConcentricRingResultSample::output(), "CROSS,0,1,-12,8,92"),
            (BlackRingResultSample::output(), "RING,1,-5,11,88"),
            (LetterResultSample::output(), "A"),
            (ColorBlockResultSample::output(), "BLOCK,blue,1,-20,15"),
        ];
        for (sample, expected) in samples {
            assert_eq!(sample["value"], expected);
            assert_eq!(sample["sample"], true);
        }
    }
}
