use crate::connection::{Connection, IncomingMessage, OutgoingMessage, Throughput};
use std::cell::RefCell;

pub trait ValueFormatter {
    fn format_value(&self, value: f64) -> String;
    fn format_throughput(&self, throughput: &Throughput, value: f64) -> String;
    fn scale_values(&self, typical_value: f64, values: &mut [f64]) -> String;
    fn scale_throughputs(
        &self,
        typical_value: f64,
        throughput: &Throughput,
        values: &mut [f64],
    ) -> String;
    fn scale_for_machines(&self, values: &mut [f64]) -> String;
}

pub struct ConnectionValueFormatter<'a> {
    connection: RefCell<&'a mut Connection>,
}
impl<'a> ConnectionValueFormatter<'a> {
    pub fn new(conn: &mut Connection) -> ConnectionValueFormatter {
        ConnectionValueFormatter {
            connection: RefCell::new(conn),
        }
    }
}
impl<'a> ValueFormatter for ConnectionValueFormatter<'a> {
    fn format_value(&self, value: f64) -> String {
        self.connection
            .borrow_mut()
            .send(&OutgoingMessage::FormatValue { value })
            .unwrap();
        match self.connection.borrow_mut().recv().unwrap().unwrap() {
            IncomingMessage::FormattedValue { value } => value,
            other => panic!("Unexpected message {:?}", other),
        }
    }

    fn format_throughput(&self, throughput: &Throughput, value: f64) -> String {
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

    fn scale_values(&self, typical_value: f64, values: &mut [f64]) -> String {
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

    fn scale_throughputs(
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

    fn scale_for_machines(&self, values: &mut [f64]) -> String {
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
impl<'a> Drop for ConnectionValueFormatter<'a> {
    fn drop(&mut self) {
        self.connection
            .borrow_mut()
            .send(&OutgoingMessage::Continue);
    }
}
