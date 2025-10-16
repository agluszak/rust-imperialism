// Core types and structs
pub mod types;
pub use types::{RecruitmentCapacity, Worker, WorkerHealth, WorkerSkill, Workforce};

// General workforce systems
pub mod systems;
pub use systems::{calculate_recruitment_cap, update_labor_pools};

// Recruitment systems
pub mod recruitment;
pub use recruitment::{
    RecruitWorkers, RecruitmentQueue, execute_recruitment_orders, handle_recruitment,
};

// Training systems
pub mod training;
pub use training::{TrainWorker, TrainingQueue, execute_training_orders, handle_training};

// Food consumption systems
pub mod consumption;
pub use consumption::feed_workers;
