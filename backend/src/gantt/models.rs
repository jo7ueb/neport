use serde::{Deserialize, Serialize};

use crate::tickets::models::{Ticket, TicketDependency};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GanttData {
    pub tickets: Vec<Ticket>,
    pub dependencies: Vec<TicketDependency>,
}
