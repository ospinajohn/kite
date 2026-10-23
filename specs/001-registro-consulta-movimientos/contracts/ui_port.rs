//! UiPort — Contrato de eventos y notificaciones de la interfaz de usuario
//!
//! Define la frontera entre la lógica de aplicación (use cases) y la capa de presentación.
//! Permite tests headless y swap de frontend (Leptos, Tauri webview, CLI, TUI).

use crate::models::movement::{Movement, MovementType};
use crate::models::summary::PeriodSummary;
use crate::contracts::parser_port::ParsedProposal;
use chrono::NaiveDate;
use std::error::Error;
use uuid::Uuid;

/// Eventos que la UI emite hacia la lógica de aplicación
pub trait UiEventHandler: Send + Sync {
    /// Usuario pide registrar un gasto por formulario
    fn on_new_expense(&self, monto_centavos: i64, concepto: String, fecha: NaiveDate);

    /// Usuario pide registrar un ingreso por formulario
    fn on_new_income(&self, monto_centavos: i64, concepto: String, fecha: NaiveDate);

    /// Usuario envía mensaje para interpretación
    fn on_parse_message(&self, text: String);

    /// Usuario confirma una propuesta de mensaje interpretado
    fn on_confirm_proposal(&self, proposal: ParsedProposal);

    /// Usuario cancela una propuesta
    fn on_cancel_proposal(&self);

    /// Usuario abre historial (opcional con rango)
    fn on_open_history(&self, desde: Option<NaiveDate>, hasta: Option<NaiveDate>);

    /// Usuario cambia rango de fechas en historial
    fn on_history_range_changed(&self, desde: Option<NaiveDate>, hasta: Option<NaiveDate>);

    /// Usuario abre resumen (opcional con rango)
    fn on_open_summary(&self, desde: Option<NaiveDate>, hasta: Option<NaiveDate>);

    /// Usuario cambia rango en resumen
    fn on_summary_range_changed(&self, desde: Option<NaiveDate>, hasta: Option<NaiveDate>);

    /// Usuario pide editar un movimiento
    fn on_edit_movement(&self, id: Uuid, monto_centavos: i64, concepto: String, fecha: NaiveDate);

    /// Usuario pide eliminar un movimiento (tras confirmación)
    fn on_delete_movement(&self, id: Uuid);

    /// Usuario cierra la app (trigger shutdown limpio)
    fn on_shutdown(&self);
}

/// Notificaciones que la lógica envía a la UI
pub trait UiNotifier: Send + Sync {
    /// Movimiento guardado correctamente
    fn show_movement_saved(&self, movement: &Movement);

    /// Error de validación al guardar
    fn show_validation_error(&self, field: &str, message: &str);

    /// Propuesta generada desde mensaje (para revisión)
    fn show_proposal(&self, proposal: &ParsedProposal);

    /// Historial actualizado
    fn show_history(&self, movements: &[Movement], desde: Option<NaiveDate>, hasta: Option<NaiveDate>);

    /// Resumen actualizado
    fn show_summary(&self, summary: &PeriodSummary);

    /// Diálogo de confirmación (blocking) — retorna true si usuario confirma
    fn ask_confirmation(&self, title: &str, message: &str) -> bool;

    /// Error genérico
    fn show_error(&self, message: &str);

    /// Info/estado (ej. "Cargando...", "Guardado")
    fn show_info(&self, message: &str);
}

/// Comando de aplicación (use case) — orquestador sin estado de UI
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
    use crate::models::movement::{Movement, MovementType, MovementOrigin};
    use crate::models::summary::PeriodSummary;
    use chrono::{Local, NaiveDate};
    use std::sync::{Arc, Mutex};
    use uuid::Uuid;

    /// Mock notifier para tests headless
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
            // En tests: auto-confirmar
            self.confirmations.lock().unwrap().push((title.to_string(), msg.to_string(), true));
            true
        }
        fn show_error(&self, msg: &str) {
            self.errors.lock().unwrap().push(msg.to_string());
        }
        fn show_info(&self, _: &str) {}
    }

    /// Test de contrato: flujo completo headless
    pub fn test_ui_flow<C: AppCommand>(cmd: &C, notifier: &MockNotifier) {
        let today = Local::now().date_naive();

        // 1. Registrar gasto por formulario
        cmd.handle_new_expense(18_000, "Almuerzo".into(), today);
        assert!(notifier.saved.lock().unwrap().is_some());
        let saved = notifier.saved.lock().unwrap().take().unwrap();
        assert_eq!(saved.tipo, MovementType::Gasto);
        assert_eq!(saved.monto_centavos, 18_000);

        // 2. Registrar ingreso
        cmd.handle_new_income(1_500_000, "Pago quincena".into(), NaiveDate::from_ymd_opt(2026,10,15).unwrap());
        assert!(notifier.saved.lock().unwrap().is_some());

        // 3. Mensaje interpretado
        cmd.handle_parse_message("gasté 12 mil en transporte".into());
        assert_eq!(notifier.proposals.lock().unwrap().len(), 1);
        let prop = notifier.proposals.lock().unwrap().pop().unwrap();
        assert_eq!(prop.monto_centavos, 12_000);

        // 4. Confirmar propuesta
        cmd.handle_confirm_proposal(prop);
        assert!(notifier.saved.lock().unwrap().is_some());

        // 5. Historial
        cmd.handle_open_history(None, None);
        assert_eq!(notifier.histories.lock().unwrap().len(), 1);
        let (movs, _, _) = notifier.histories.lock().unwrap().pop().unwrap();
        assert_eq!(movs.len(), 3); // 2 form + 1 mensaje

        // 6. Resumen
        cmd.handle_open_summary(None, None);
        assert_eq!(notifier.summaries.lock().unwrap().len(), 1);
        let sum = notifier.summaries.lock().unwrap().pop().unwrap();
        assert_eq!(sum.ingresos_centavos, 1_500_000);
        assert_eq!(sum.gastos_centavos, 30_000); // 18k + 12k

        // 7. Editar
        let id = movs[0].id;
        cmd.handle_edit_movement(id, 19_000, "Almuerzo corregido".into(), today);
        cmd.handle_open_summary(None, None);
        let sum2 = notifier.summaries.lock().unwrap().pop().unwrap();
        assert_eq!(sum2.gastos_centavos, 31_000); // 19k + 12k

        // 8. Eliminar (con confirmación automática en mock)
        cmd.handle_delete_movement(movs[1].id);
        cmd.handle_open_history(None, None);
        let (movs2, _, _) = notifier.histories.lock().unwrap().pop().unwrap();
        assert_eq!(movs2.len(), 2);
    }
}