use crate::connection::{Connection, IncomingMessage, OutgoingMessage, Throughput};
use std::cell::RefCell;

pub struct ValueFormatter<'a> {
    connection: RefCell<&'a mut Connection>,
}
impl<'a> ValueFormatter<'a> {
    pub fn new(conn: &mut Connection) -> ValueFormatter {
        ValueFormatter {
            connection: RefCell::new(conn),
        }
    }
}
impl<'a> ValueFormatter<'a> {
    pub fn format_value(&self, value: f64) -> String {
        self.connection
            .borrow_mut()
            .send(&OutgoingMessage::FormatValue { value })
            .unwrap();
        match self.connection.borrow_mut().recv().unwrap().unwrap() {
            IncomingMessage::FormattedValue { value } => value,
            other => panic!("Unexpected message {:?}", other),
        }
    }

    pub fn format_throughput(&self, throughput: &Throughput, value: f64) -> String {
        self.connection
            .borrow_mut()
            .send(&OutgoingMessage::FormatThroughput {
                value,
                throughput: throughput.clone(),
            })
            .unwrap();
        match self.connection.borrow_mut().recv().unwrap().unwrap() {
            IncomingMessage::FormattedValue { value } => value,
            other => panic!("Unexpected message {:?}", other),
        }
    }

    pub fn scale_values(&self, typical_value: f64, values: &mut [f64]) -> String {
        self.connection
            .borrow_mut()
            .send(&OutgoingMessage::ScaleValues {
                typical_value,
                values,
            })
            .unwrap();
        match self.connection.borrow_mut().recv().unwrap().unwrap() {
            IncomingMessage::ScaledValues {
                scaled_values,
                unit,
            } => {
                values.copy_from_slice(&scaled_values);
                unit
            }
            other => panic!("Unexpected message {:?}", other),
        }
    }

    // This will be needed when we add the throughput plots.
    #[allow(dead_code)]
    pub fn scale_throughputs(
        &self,
        typical_value: f64,
        throughput: &Throughput,
        values: &mut [f64],
    ) -> String {
        self.connection
            .borrow_mut()
            .send(&OutgoingMessage::ScaleThroughputs {
                typical_value,
                values,
                throughput: throughput.clone(),
            })
            .unwrap();
        match self.connection.borrow_mut().recv().unwrap().unwrap() {
            IncomingMessage::ScaledValues {
                scaled_values,
                unit,
            } => {
                values.copy_from_slice(&scaled_values);
                unit
            }
            other => panic!("Unexpected message {:?}", other),
        }
    }

    pub fn scale_for_machines(&self, values: &mut [f64]) -> String {
        self.connection
            .borrow_mut()
            .send(&OutgoingMessage::ScaleForMachines { values })
            .unwrap();
        match self.connection.borrow_mut().recv().unwrap().unwrap() {
            IncomingMessage::ScaledValues {
                scaled_values,
                unit,
            } => {
                values.copy_from_slice(&scaled_values);
                unit
            }
            other => panic!("Unexpected message {:?}", other),
        }
    }
}
impl<'a> Drop for ValueFormatter<'a> {
    fn drop(&mut self) {
        let _ = self
            .connection
            .borrow_mut()
            .send(&OutgoingMessage::Continue);
    }
}
