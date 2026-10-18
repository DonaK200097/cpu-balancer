//! Состояние редактора схемы: текущие значения, бэкап, накопленные правки.

use crate::backup;
use crate::controls::{self, Control, PV};
use crate::power::{self, Values};
use serde::Serialize;

#[derive(Default)]
pub struct Editor {
    pub scheme: String,
    pub name: String,
    pub cur: Values,
    pub bk: Values,
    pub pend: Values,
}

#[derive(Serialize, Clone, Copy, Default)]
pub struct Cell {
    /// slider/enum — значение первого ключа
    pub value: Option<i64>,
    /// mode — индекс варианта (-1 = «своё»)
    pub mode: i32,
}

#[derive(Serialize)]
pub struct CtlView {
    pub id: &'static str,
    pub available: bool,
    pub changed: bool,
    pub cur: [Cell; 2],
    pub backup: [Cell; 2],
}

#[derive(Serialize)]
pub struct EditorView {
    pub scheme: String,
    pub name: String,
    pub active: bool,
    pub pending: usize,
    pub controls: Vec<CtlView>,
}

#[derive(Serialize)]
pub struct SaveResult {
    pub saved: usize,
    pub failed: Vec<String>,
    pub active: bool,
}

impl Editor {
    pub fn open(scheme: &str, name: &str, write_backup: bool) -> Editor {
        let cur = power::read_scheme(scheme);
        let bk = backup::get_backup(scheme, name, &cur, write_backup);
        Editor { scheme: scheme.to_string(), name: name.to_string(), cur, bk, pend: Values::new() }
    }

    pub fn get(&self, k: &str, c: usize) -> Option<i64> {
        self.pend.get(k).or_else(|| self.cur.get(k)).map(|v| v[c])
    }

    fn bk_get(&self, k: &str, c: usize) -> Option<i64> {
        self.bk.get(k).or_else(|| self.cur.get(k)).map(|v| v[c])
    }

    pub fn set(&mut self, k: &str, c: usize, v: i64) {
        let Some(cur) = self.cur.get(k).copied() else { return };
        let p = self.pend.entry(k.to_string()).or_insert(cur);
        p[c] = v;
        if *p == cur {
            self.pend.remove(k);
        }
    }

    fn available(&self, ctl: &Control) -> bool {
        ctl.keys().iter().all(|k| self.cur.contains_key(*k))
    }

    fn changed(&self, ctl: &Control) -> bool {
        ctl.keys().iter().any(|k| self.pend.contains_key(*k))
    }

    /// Индекс варианта mode, совпадающего со значениями (-1 = «своё»).
    pub fn mode_idx(&self, ctl: &Control, c: usize, from_backup: bool) -> i32 {
        let Control::Mode { opts, .. } = ctl else { return -1 };
        let get = |k: &str| if from_backup { self.bk_get(k, c) } else { self.get(k, c) };
        for (i, o) in opts.iter().enumerate() {
            if o.map.is_empty() {
                continue;
            }
            if o.map.iter().all(|(k, v)| get(k) == Some(*v)) {
                return i as i32;
            }
        }
        for (i, o) in opts.iter().enumerate() {
            if !o.backup {
                continue;
            }
            if from_backup {
                return i as i32;
            }
            if ctl.keys().iter().all(|k| self.get(k, c) == self.bk_get(k, c)) {
                return i as i32;
            }
        }
        -1
    }

    fn set_mode_idx(&mut self, ctl: &Control, c: usize, i: usize) {
        let Control::Mode { opts, .. } = ctl else { return };
        let Some(o) = opts.get(i) else { return };
        if !o.map.is_empty() {
            for (k, v) in o.map.iter() {
                self.set(k, c, *v);
            }
        } else {
            for k in ctl.keys() {
                if let Some(v) = self.bk_get(k, c) {
                    self.set(k, c, v);
                }
            }
        }
    }

    /// slider/enum — значение; mode — индекс варианта.
    pub fn set_control(&mut self, id: &str, c: usize, value: i64) {
        let Some(ctl) = controls::find(id) else { return };
        if !self.available(&ctl) || c > 1 {
            return;
        }
        match &ctl {
            Control::Slider { keys, min, max, zero, .. } => {
                let v = if *zero && value == 0 { 0 } else { value.clamp(*min, *max) };
                for k in keys.iter() {
                    self.set(k, c, v);
                }
            }
            Control::Enum { keys, .. } => {
                for k in keys.iter() {
                    self.set(k, c, value);
                }
            }
            Control::Mode { .. } => self.set_mode_idx(&ctl, c, value.max(0) as usize),
        }
    }

    pub fn copy_to_other(&mut self, id: &str, c: usize) {
        let Some(ctl) = controls::find(id) else { return };
        if !self.available(&ctl) || c > 1 {
            return;
        }
        let o = 1 - c;
        if let Control::Mode { .. } = ctl {
            let i = self.mode_idx(&ctl, c, false);
            if i >= 0 {
                self.set_mode_idx(&ctl, o, i as usize);
            }
        } else {
            for k in ctl.keys() {
                if let Some(v) = self.get(k, c) {
                    self.set(k, o, v);
                }
            }
        }
    }

    pub fn apply_preset(&mut self, id: &str) -> bool {
        let Some(pr) = controls::presets().into_iter().find(|p| p.id == id) else { return false };
        for (cid, vals) in &pr.vals {
            let Some(ctl) = controls::find(cid) else { continue };
            if !self.available(&ctl) {
                continue;
            }
            for (c, v) in vals.iter().enumerate() {
                match (v, &ctl) {
                    (Some(PV::M(opt)), Control::Mode { opts, .. }) => {
                        if let Some(i) = opts.iter().position(|o| o.id == *opt) {
                            self.set_mode_idx(&ctl, c, i);
                        }
                    }
                    (Some(PV::N(n)), _) => {
                        for k in ctl.keys() {
                            self.set(k, c, *n);
                        }
                    }
                    _ => {}
                }
            }
        }
        for (k, vals) in &pr.raw {
            for (c, v) in vals.iter().enumerate() {
                if let Some(v) = v {
                    self.set(k, c, *v);
                }
            }
        }
        true
    }

    /// «Исходные»: подставить значения из бэкапа во все ключи.
    pub fn restore_backup(&mut self) {
        let bk: Vec<(String, [i64; 2])> = self.bk.iter().map(|(k, v)| (k.clone(), *v)).collect();
        for (k, v) in bk {
            for c in 0..2 {
                self.set(&k, c, v[c]);
            }
        }
    }

    pub fn discard(&mut self) {
        self.pend.clear();
    }

    pub fn save(&mut self) -> SaveResult {
        let mut saved = 0;
        let mut failed = Vec::new();
        let pend: Vec<(String, [i64; 2])> = self.pend.iter().map(|(k, v)| (k.clone(), *v)).collect();
        for (k, v) in pend {
            let cur = self.cur.get(&k).copied().unwrap_or([i64::MIN; 2]);
            for c in 0..2 {
                if v[c] != cur[c] {
                    if power::write_value(&self.scheme, &k, c, v[c]) {
                        saved += 1;
                    } else {
                        failed.push(format!("{k}/{}", if c == 0 { "AC" } else { "DC" }));
                    }
                }
            }
        }
        // запись в активную схему применяется только после повторной активации
        let active = power::active_scheme().map(|g| power::guid_str(&g)) == Some(self.scheme.clone());
        if active {
            power::set_active(&self.scheme);
        }
        self.cur = power::read_scheme(&self.scheme);
        self.pend.clear();
        SaveResult { saved, failed, active }
    }

    pub fn view(&self) -> EditorView {
        let active = power::active_scheme().map(|g| power::guid_str(&g)) == Some(self.scheme.clone());
        let mut controls_v = Vec::new();
        for g in controls::groups() {
            for ctl in g.controls {
                let available = self.available(&ctl);
                let mut cur = [Cell::default(); 2];
                let mut backup = [Cell::default(); 2];
                if available {
                    for c in 0..2 {
                        match &ctl {
                            Control::Mode { .. } => {
                                cur[c].mode = self.mode_idx(&ctl, c, false);
                                backup[c].mode = self.mode_idx(&ctl, c, true);
                            }
                            _ => {
                                let k = ctl.keys()[0];
                                cur[c].value = self.get(k, c);
                                backup[c].value = self.bk_get(k, c);
                            }
                        }
                    }
                }
                controls_v.push(CtlView { id: ctl.id(), available, changed: self.changed(&ctl), cur, backup });
            }
        }
        EditorView {
            scheme: self.scheme.clone(),
            name: self.name.clone(),
            active,
            pending: self.pend.len(),
            controls: controls_v,
        }
    }
}
