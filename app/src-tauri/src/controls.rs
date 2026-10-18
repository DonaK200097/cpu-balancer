//! Модель элементов интерфейса и пресетов (порт `$Controls` / `$Presets` из v1).
//! Подписи и подсказки — во фронтенде (locales/*.json) по `id` элемента и `id` варианта.

use serde::Serialize;

#[derive(Serialize, Clone, Debug)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Control {
    Slider {
        id: &'static str,
        keys: &'static [&'static str],
        min: i64,
        max: i64,
        step: i64,
        fine: i64,
        big: i64,
        unit: &'static str,
        /// 0 = «без лимита» (рисуется у правого края)
        zero: bool,
    },
    Enum {
        id: &'static str,
        keys: &'static [&'static str],
        /// значения от «быстрее/горячее» к «экономнее/холоднее»
        opts: &'static [i64],
    },
    Mode {
        id: &'static str,
        opts: &'static [ModeOpt],
    },
}

#[derive(Serialize, Clone, Debug)]
pub struct ModeOpt {
    pub id: &'static str,
    /// пусто + backup = «Как было» (значения из бэкапа)
    pub map: &'static [(&'static str, i64)],
    pub backup: bool,
}

#[derive(Serialize, Clone, Debug)]
pub struct Group {
    pub id: &'static str,
    pub controls: Vec<Control>,
}

impl Control {
    pub fn id(&self) -> &'static str {
        match self {
            Control::Slider { id, .. } | Control::Enum { id, .. } | Control::Mode { id, .. } => id,
        }
    }

    /// Все ключи, которые трогает элемент.
    pub fn keys(&self) -> Vec<&'static str> {
        match self {
            Control::Slider { keys, .. } | Control::Enum { keys, .. } => keys.to_vec(),
            Control::Mode { opts, .. } => {
                let mut ks: Vec<&'static str> = Vec::new();
                for o in opts.iter() {
                    for (k, _) in o.map.iter() {
                        if !ks.contains(k) {
                            ks.push(k);
                        }
                    }
                }
                ks
            }
        }
    }
}

pub const HYDROC_ZONE: &[(&str, i64)] =
    &[("CONTPOL", 1), ("HYBCU", 50), ("HYBCC", 60), ("EFFCU", 50), ("EFFCC", 60), ("CONTINC", 2), ("CONTDEC", 5)];

pub fn groups() -> Vec<Group> {
    use Control::*;
    vec![
        Group {
            id: "main",
            controls: vec![
                Slider { id: "eppP", keys: &["EPP1"], min: 0, max: 100, step: 5, fine: 1, big: 20, unit: "", zero: false },
                Slider { id: "eppE", keys: &["EPP"], min: 0, max: 100, step: 5, fine: 1, big: 20, unit: "", zero: false },
                Enum { id: "boost", keys: &["BOOST"], opts: &[2, 4, 1, 3, 0] },
                Slider { id: "maxstate", keys: &["MAXSTATE"], min: 50, max: 100, step: 5, fine: 1, big: 10, unit: "%", zero: false },
                Slider { id: "freqP", keys: &["FREQ1"], min: 0, max: 6000, step: 100, fine: 100, big: 500, unit: "MHz", zero: true },
                Slider { id: "freqE", keys: &["FREQ"], min: 0, max: 6000, step: 100, fine: 100, big: 500, unit: "MHz", zero: true },
                Slider { id: "minstate", keys: &["MINSTATE", "MINSTATE1"], min: 0, max: 100, step: 5, fine: 1, big: 20, unit: "%", zero: false },
            ],
        },
        Group {
            id: "cores",
            controls: vec![
                Slider { id: "park", keys: &["CPMIN", "CPMIN1"], min: 0, max: 100, step: 5, fine: 1, big: 25, unit: "%", zero: false },
                Enum { id: "sched", keys: &["SCHED", "SHORT"], opts: &[1, 2, 0, 5, 4, 3] },
                Mode {
                    id: "contain",
                    opts: &[
                        ModeOpt { id: "backup", map: &[], backup: true },
                        ModeOpt { id: "hydroc", map: HYDROC_ZONE, backup: false },
                    ],
                },
            ],
        },
        Group {
            id: "misc",
            controls: vec![
                Mode {
                    id: "latency",
                    opts: &[
                        ModeOpt { id: "fast", map: &[("LATPERF", 100), ("LATUNPARK", 50), ("INITC1", 100)], backup: false },
                        ModeOpt { id: "eco", map: &[("LATPERF", 0), ("LATUNPARK", 0), ("INITC1", 0)], backup: false },
                        ModeOpt { id: "backup", map: &[], backup: true },
                    ],
                },
                Enum { id: "hwp", keys: &["AUTON"], opts: &[1, 0] },
                Enum { id: "aspm", keys: &["ASPM"], opts: &[0, 1, 2] },
                Enum { id: "cooling", keys: &["COOLING"], opts: &[1, 0] },
            ],
        },
    ]
}

pub fn find(id: &str) -> Option<Control> {
    groups().into_iter().flat_map(|g| g.controls).find(|c| c.id() == id)
}

/// Значение пресета для элемента: число (slider/enum) или id варианта (mode).
#[derive(Serialize, Clone, Copy, Debug)]
#[serde(untagged)]
pub enum PV {
    N(i64),
    M(&'static str),
}

#[derive(Serialize, Clone, Debug)]
pub struct Preset {
    pub id: &'static str,
    /// элемент → [от сети, от батареи]; None = не трогать
    pub vals: Vec<(&'static str, [Option<PV>; 2])>,
    /// скрытые ключи без элементов UI
    pub raw: Vec<(&'static str, [Option<i64>; 2])>,
}

const fn n(v: i64) -> Option<PV> {
    Some(PV::N(v))
}
const fn m(v: &'static str) -> Option<PV> {
    Some(PV::M(v))
}

fn hydroc_extra() -> Vec<(&'static str, [Option<i64>; 2])> {
    vec![
        ("FLOORC0", [Some(0), Some(0)]),
        ("IDLEPROMOTE", [Some(0), Some(0)]),
        ("IDLESCALE", [Some(1), Some(1)]),
        ("HETERO", [Some(4), Some(4)]),
    ]
}

/// Пресеты только подставляют значения в правки. Лимиты частоты не трогают.
pub fn presets() -> Vec<Preset> {
    vec![
        Preset {
            id: "quiet",
            vals: vec![
                ("eppP", [n(70), n(85)]),
                ("eppE", [n(80), n(90)]),
                ("boost", [n(3), n(3)]),
                ("maxstate", [n(100), n(100)]),
                ("sched", [n(5), n(4)]),
                ("contain", [m("hydroc"), m("hydroc")]),
                ("latency", [m("eco"), m("eco")]),
                ("hwp", [n(1), n(1)]),
                ("aspm", [n(1), n(2)]),
                ("cooling", [n(0), n(0)]),
            ],
            raw: hydroc_extra(),
        },
        Preset {
            id: "hydroc",
            vals: vec![
                ("eppP", [n(64), n(80)]),
                ("eppE", [n(80), n(80)]),
                ("boost", [n(2), n(3)]),
                ("maxstate", [n(100), n(100)]),
                ("park", [n(100), None]),
                ("contain", [m("hydroc"), m("hydroc")]),
                ("latency", [m("eco"), m("eco")]),
                ("hwp", [n(1), n(1)]),
            ],
            raw: hydroc_extra(),
        },
        Preset {
            id: "hydrocSoft",
            vals: vec![
                ("eppP", [n(48), n(65)]),
                ("eppE", [n(56), n(65)]),
                ("boost", [n(2), n(3)]),
                ("maxstate", [n(100), n(100)]),
                ("park", [n(100), None]),
                ("contain", [m("hydroc"), m("hydroc")]),
                ("latency", [m("eco"), m("eco")]),
                ("hwp", [n(1), n(1)]),
            ],
            raw: hydroc_extra(),
        },
        Preset {
            id: "perf",
            vals: vec![
                ("eppP", [n(15), n(40)]),
                ("eppE", [n(25), n(50)]),
                ("boost", [n(2), n(4)]),
                ("maxstate", [n(100), n(100)]),
                ("park", [n(100), None]),
                ("sched", [n(5), n(5)]),
                ("contain", [m("backup"), m("backup")]),
                ("latency", [m("fast"), m("fast")]),
                ("hwp", [n(1), n(1)]),
                ("cooling", [n(1), None]),
            ],
            raw: vec![
                ("FLOORC0", [Some(100), Some(100)]),
                ("IDLEPROMOTE", [Some(60), Some(60)]),
                ("IDLESCALE", [Some(0), Some(0)]),
            ],
        },
        Preset {
            id: "battery",
            vals: vec![
                ("eppP", [None, n(90)]),
                ("eppE", [None, n(95)]),
                ("boost", [None, n(0)]),
                ("sched", [None, n(4)]),
                ("contain", [None, m("hydroc")]),
                ("latency", [None, m("eco")]),
                ("hwp", [None, n(1)]),
                ("aspm", [None, n(2)]),
                ("cooling", [None, n(0)]),
            ],
            raw: vec![],
        },
    ]
}
