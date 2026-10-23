use crate::models::{Movement, MovementType};
use crate::parser::port::ParsedProposal;
use crate::models::summary::PeriodSummary;
use chrono::NaiveDate;
use std::error::Error;
use uuid::Uuid;

pub trait UiEventHandler: Send + Sync {
    fn on_new_expense(&self, monto_centavos: i64, concepto: String, fecha: NaiveDate);
    fn on_new_income(&self, monto_centavos: i64, concepto: String, fecha: NaiveDate);
    fn on_parse_message(&self, text: String);
    fn on_confirm_proposal(&self, proposal: ParsedProposal);
    fn on_cancel_proposal(&self);
    fn on_open_history(&self, desde: Option<NaiveDate>, hasta: Option<NaiveDate>);
    fn on_history_range_changed(&self, desde: Option<NaiveDate>, hasta: Option<NaiveDate>);
    fn on_open_summary(&self, desde: Option<NaiveDate>, hasta: Option<NaiveDate>);
    fn on_summary_range_changed(&self, desde: Option<NaiveDate>, hasta: Option<NaiveDate>);
    fn on_edit_movement(&self, id: Uuid, monto_centavos: i64, concepto: String, fecha: NaiveDate);
    fn on_delete_movement(&self, id: Uuid);
    fn on_shutdown(&self);
}

pub trait UiNotifier: Send + Sync {
    fn show_movement_saved(&self, movement: &Movement);
    fn show_validation_error(&self, field: &str, message: &str);
    fn show_proposal(&self, proposal: &ParsedProposal);
    fn show_history(&self, movements: &[Movement], desde: Option<NaiveDate>, hasta: Option<NaiveDate>);
    fn show_summary(&self, summary: &PeriodSummary);
    fn ask_confirmation(&self, title: &str, message: &str) -> bool;
    fn show_error(&self, message: &str);
    fn show_info(&self, message: &str);
}

pub trait AppCommand: Send + Sync {
    fn handle_new_expense(&self, monto_centavos: i64, concepto: String, fecha: NaiveDate);
    fn handle_new_income(&self, monto_centavos: i64, concepto: String, fecha: NaiveDate);
    fn handle_parse_message(&self, text: String);
    fn handle_confirm_proposal(&self, proposal: ParsedProposal);
    fn handle_open_history(&self, desde: Option<NaiveDate>, hasta: Option<NaiveDate>);
    fn handle_open_summary(&self, desde: Option<NaiveDate>, hasta: Option<NaiveDate>);
    fn handle_edit_movement(&self, id: Uuid, monto_centavos: i64, concepto: String, fecha: NaiveDate);
    fn handle_delete_movement(&self, id: Uuid);
    fn shutdown(&self);
}

#[cfg(test)]
mod contract_tests {
    use super::*;
    use crate::models::{Movement, MovementType, MovementOrigin};
    use crate::models::summary::PeriodSummary;
    use chrono::{Local, NaiveDate};
    use std::sync::{Arc, Mutex};
    use uuid::Uuid;

    #[derive(Default)]
    struct MockNotifier {
        pub saved: Mutex<Option<Movement>>,
        pub proposals: Mutex<Vec<ParsedProposal>>,
        pub histories: Mutex<Vec<(Vec<Movement>, Option<NaiveDate>, Option<NaiveDate>)>>,
        pub summaries: Mutex<Vec<PeriodSummary>>,
        pub confirmations: Mutex<Vec<(String, String, bool)>>,
        pub errors: Mutex<Vec<String>>,
    }

    impl UiNotifier for MockNotifier {
        fn show_movement_saved(&self, m: &Movement) {
            self.saved.lock().unwrap().replace(m.clone());
        }
        fn show_validation_error(&self, _: &str, _: &str) {}
        fn show_proposal(&self, p: &ParsedProposal) {
            self.proposals.lock().unwrap().push(p.clone());
        }
        fn show_history(&self, m: &[Movement], d: Option<NaiveDate>, h: Option<NaiveDate>) {
            self.histories.lock().unwrap().push((m.to_vec(), d, h));
        }
        fn show_summary(&self, s: &PeriodSummary) {
            self.summaries.lock().unwrap().push(s.clone());
        }
        fn ask_confirmation(&self, title: &str, msg: &str) -> bool {
            self.confirmations.lock().unwrap().push((title.to_string(), msg.to_string(), true));
            true
        }
        fn show_error(&self, msg: &str) {
            self.errors.lock().unwrap().push(msg.to_string());
        }
        fn show_info(&self, _: &str) {}
    }

    pub fn test_ui_flow<C: AppCommand>(cmd: &C, notifier: &MockNotifier) {
        let today = Local::now().date_naive();

        cmd.handle_new_expense(18_000, "Almuerzo".into(), today);
        assert!(notifier.saved.lock().unwrap().is_some());
        let saved = notifier.saved.lock().unwrap().take().unwrap();
        assert_eq!(saved.tipo, MovementType::Gasto);
        assert_eq!(saved.monto_centavos, 18_000);

        cmd.handle_new_income(1_500_000, "Pago quincena".into(), NaiveDate::from_ymd_opt(2026,10,15).unwrap());
        assert!(notifier.saved.lock().unwrap().is_some());

        cmd.handle_parse_message("gasté 12 mil en transporte".into());
        assert_eq!(notifier.proposals.lock().unwrap().len(), 1);
        let prop = notifier.proposals.lock().unwrap().pop().unwrap();
        assert_eq!(prop.monto_centavos, 12_000);

        cmd.handle_confirm_proposal(prop);
        assert!(notifier.saved.lock().unwrap().is_some());

        cmd.handle_open_history(None, None);
        assert_eq!(notifier.histories.lock().unwrap().len(), 1);
        let (movs, _, _) = notifier.histories.lock().unwrap().pop().unwrap();
        assert_eq!(movs.len(), 3);

        cmd.handle_open_summary(None, None);
        assert_eq!(notifier.summaries.lock().unwrap().len(), 1);
        let sum = notifier.summaries.lock().unwrap().pop().unwrap();
        assert_eq!(sum.ingresos_centavos, 1_500_000);
        assert_eq!(sum.gastos_centavos, 30_000);

        let id = movs[0].id;
        cmd.handle_edit_movement(id, 19_000, "Almuerzo corregido".into(), today);
        cmd.handle_open_summary(None, None);
        let sum2 = notifier.summaries.lock().unwrap().pop().unwrap();
        assert_eq!(sum2.gastos_centavos, 31_000);

        cmd.handle_delete_movement(movs[1].id);
        cmd.handle_open_history(None, None);
        let (movs2, _, _) = notifier.histories.lock().unwrap().pop().unwrap();
        assert_eq!(movs2.len(), 2);
    }
}