/// An intermediary state of an import task. 
/// It carries over the title and content of a task. 
#[derive(Debug, Clone)]
pub struct ImportTaskIntermediate {
    pub title: String,
    pub content: String,
}