//! Virtual controller: button vocabulary shared by the UI and the CLI.
//!
//! The actual presses go through webMAN MOD's `/pad.ps3?` command (see
//! `webman`), which drives the console's virtual pad. This module owns the
//! key names, combo parsing and the press-hold-release sequence semantics.

use crate::error::{Error, Result};
use crate::webman::WebMan;
use std::time::Duration;

/// Every button webMAN accepts by name.
pub const BUTTONS: &[&str] = &[
    "cross", "circle", "square", "triangle", "up", "down", "left", "right", "l1", "l2", "l3", "r1",
    "r2", "r3", "select", "start", "ps",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Button {
    Cross,
    Circle,
    Square,
    Triangle,
    Up,
    Down,
    Left,
    Right,
    L1,
    L2,
    L3,
    R1,
    R2,
    R3,
    Select,
    Start,
    Ps,
}

impl Button {
    pub fn parse(name: &str) -> Result<Button> {
        Ok(match name.trim().to_ascii_lowercase().as_str() {
            "cross" | "x" => Button::Cross,
            "circle" | "o" => Button::Circle,
            "square" => Button::Square,
            "triangle" => Button::Triangle,
            "up" => Button::Up,
            "down" => Button::Down,
            "left" => Button::Left,
            "right" => Button::Right,
            "l1" => Button::L1,
            "l2" => Button::L2,
            "l3" => Button::L3,
            "r1" => Button::R1,
            "r2" => Button::R2,
            "r3" => Button::R3,
            "select" => Button::Select,
            "start" => Button::Start,
            "ps" => Button::Ps,
            other => return Err(Error::Invalid(format!("unknown button '{other}'"))),
        })
    }

    pub fn name(self) -> &'static str {
        match self {
            Button::Cross => "cross",
            Button::Circle => "circle",
            Button::Square => "square",
            Button::Triangle => "triangle",
            Button::Up => "up",
            Button::Down => "down",
            Button::Left => "left",
            Button::Right => "right",
            Button::L1 => "l1",
            Button::L2 => "l2",
            Button::L3 => "l3",
            Button::R1 => "r1",
            Button::R2 => "r2",
            Button::R3 => "r3",
            Button::Select => "select",
            Button::Start => "start",
            Button::Ps => "ps",
        }
    }
}

/// Press one button once. Rejects names webMAN does not know, so a typo in the
/// UI or a macro file fails loudly instead of silently doing nothing.
pub fn press(wm: &WebMan, name: &str) -> Result<()> {
    let button = Button::parse(name)?;
    wm.pad(button.name())
}

/// Hold a button until `release` is called.
pub fn hold(wm: &WebMan, name: &str) -> Result<()> {
    let button = Button::parse(name)?;
    wm.pad_hold(button.name())
}

pub fn release(wm: &WebMan) -> Result<()> {
    wm.pad_release()
}

/// Press buttons together: hold all of them for `hold_ms`, then release.
pub fn combo(wm: &WebMan, names: &[&str], hold_ms: u64) -> Result<()> {
    if names.is_empty() {
        return Ok(());
    }
    let mut buttons = Vec::with_capacity(names.len());
    for name in names {
        buttons.push(Button::parse(name)?);
    }
    let keys: Vec<&str> = buttons.iter().map(|b| b.name()).collect();
    wm.pad_combo_hold(&keys)?;
    std::thread::sleep(Duration::from_millis(hold_ms.clamp(40, 5_000)));
    wm.pad_release()
}

/// Run a macro: each step is a button name or a `a+b` combo, separated by
/// commas. Useful for XMB navigation shortcuts.
pub fn macro_run(wm: &WebMan, script: &str, step_ms: u64) -> Result<()> {
    let step = Duration::from_millis(step_ms.clamp(20, 5_000));
    for step_text in script.split(',') {
        let step_text = step_text.trim();
        if step_text.is_empty() {
            continue;
        }
        let parts: Vec<&str> = step_text
            .split('+')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();
        if parts.is_empty() {
            continue;
        }
        let mut buttons = Vec::with_capacity(parts.len());
        for name in &parts {
            buttons.push(Button::parse(name)?);
        }
        let keys: Vec<&str> = buttons.iter().map(|b| b.name()).collect();
        wm.pad_combo_hold(&keys)?;
        std::thread::sleep(step);
        wm.pad_release()?;
        std::thread::sleep(step);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_aliases_and_names() {
        assert_eq!(Button::parse("X").unwrap(), Button::Cross);
        assert_eq!(Button::parse("Circle").unwrap(), Button::Circle);
        assert_eq!(Button::parse("l3").unwrap().name(), "l3");
        assert!(Button::parse("jump").is_err());
    }

    #[test]
    fn every_button_round_trips() {
        for name in BUTTONS {
            assert_eq!(Button::parse(name).unwrap().name(), *name);
        }
    }
}
