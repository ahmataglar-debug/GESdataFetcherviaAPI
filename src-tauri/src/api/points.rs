#[derive(Debug, Clone, Copy)]
pub struct StringPoint {
    pub position: usize,
    pub current: &'static str,
    pub voltage: &'static str,
}

pub const PLANT_ACTIVE_POWER: &str = "83033";
pub const INVERTER_ACTIVE_POWER: &str = "24";

pub const STRING_POINTS: [StringPoint; 40] = [
    StringPoint { position: 1, current: "70", voltage: "96" },
    StringPoint { position: 2, current: "71", voltage: "97" },
    StringPoint { position: 3, current: "72", voltage: "98" },
    StringPoint { position: 4, current: "73", voltage: "99" },
    StringPoint { position: 5, current: "74", voltage: "100" },
    StringPoint { position: 6, current: "75", voltage: "101" },
    StringPoint { position: 7, current: "76", voltage: "102" },
    StringPoint { position: 8, current: "77", voltage: "103" },
    StringPoint { position: 9, current: "78", voltage: "104" },
    StringPoint { position: 10, current: "79", voltage: "105" },
    StringPoint { position: 11, current: "80", voltage: "106" },
    StringPoint { position: 12, current: "81", voltage: "107" },
    StringPoint { position: 13, current: "82", voltage: "108" },
    StringPoint { position: 14, current: "83", voltage: "109" },
    StringPoint { position: 15, current: "84", voltage: "110" },
    StringPoint { position: 16, current: "85", voltage: "111" },
    StringPoint { position: 17, current: "92", voltage: "112" },
    StringPoint { position: 18, current: "93", voltage: "113" },
    StringPoint { position: 19, current: "313", voltage: "7166" },
    StringPoint { position: 20, current: "314", voltage: "7167" },
    StringPoint { position: 21, current: "315", voltage: "7168" },
    StringPoint { position: 22, current: "316", voltage: "7169" },
    StringPoint { position: 23, current: "317", voltage: "7170" },
    StringPoint { position: 24, current: "318", voltage: "7171" },
    StringPoint { position: 25, current: "319", voltage: "7172" },
    StringPoint { position: 26, current: "320", voltage: "7173" },
    StringPoint { position: 27, current: "321", voltage: "7174" },
    StringPoint { position: 28, current: "322", voltage: "7175" },
    StringPoint { position: 29, current: "323", voltage: "7176" },
    StringPoint { position: 30, current: "324", voltage: "7177" },
    StringPoint { position: 31, current: "325", voltage: "7178" },
    StringPoint { position: 32, current: "326", voltage: "7179" },
    StringPoint { position: 33, current: "7708", voltage: "7707" },
    StringPoint { position: 34, current: "7710", voltage: "7709" },
    StringPoint { position: 35, current: "7712", voltage: "7711" },
    StringPoint { position: 36, current: "7714", voltage: "7713" },
    StringPoint { position: 37, current: "7716", voltage: "7715" },
    StringPoint { position: 38, current: "7718", voltage: "7717" },
    StringPoint { position: 39, current: "7720", voltage: "7719" },
    StringPoint { position: 40, current: "7722", voltage: "7721" },
];

pub fn inverter_point_batches() -> [Vec<String>; 2] {
    let first = std::iter::once(INVERTER_ACTIVE_POWER.to_string())
        .chain(STRING_POINTS[..20].iter().flat_map(|point| [point.current.to_string(), point.voltage.to_string()]))
        .collect();
    let second = STRING_POINTS[20..]
        .iter()
        .flat_map(|point| [point.current.to_string(), point.voltage.to_string()])
        .collect();
    [first, second]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_string_has_unique_current_and_voltage_points() {
        let mut ids = std::collections::BTreeSet::new();
        for (index, point) in STRING_POINTS.iter().enumerate() {
            assert_eq!(point.position, index + 1);
            assert!(ids.insert(point.current));
            assert!(ids.insert(point.voltage));
        }
    }

    #[test]
    fn realtime_batches_stay_below_the_api_limit() {
        for batch in inverter_point_batches() {
            assert!(batch.len() <= 50);
        }
    }
}
