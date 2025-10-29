use b4n_config::keys::KeyCombination;
use crossterm::event::KeyCode;
use std::{cell::RefCell, rc::Rc};

use crate::core::AppData;

use super::*;

#[test]
fn esc_reverts_value_test() {
    let data = Rc::new(RefCell::new(AppData::default()));
    let mut filter = Filter::new(data, None, 60);

    filter.show();
    filter.process_event(&TuiEvent::Key(KeyCombination::from('t')));
    filter.process_event(&TuiEvent::Key(KeyCombination::from('e')));
    filter.process_event(&TuiEvent::Key(KeyCombination::from('s')));
    filter.process_event(&TuiEvent::Key(KeyCombination::from('t')));

    assert_eq!("test", filter.value());

    filter.process_event(&TuiEvent::Key(KeyCombination::from(KeyCode::Esc)));

    assert_eq!("", filter.value());
}

#[test]
fn enter_stores_value_test() {
    let data = Rc::new(RefCell::new(AppData::default()));
    let mut filter = Filter::new(data, None, 60);

    filter.show();
    filter.process_event(&TuiEvent::Key(KeyCombination::from('t')));
    filter.process_event(&TuiEvent::Key(KeyCombination::from('e')));
    filter.process_event(&TuiEvent::Key(KeyCombination::from('s')));
    filter.process_event(&TuiEvent::Key(KeyCombination::from('t')));

    assert_eq!("test", filter.value());

    filter.process_event(&TuiEvent::Key(KeyCombination::from(KeyCode::Enter)));

    assert_eq!("test", filter.value());
}
