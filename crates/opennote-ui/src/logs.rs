use fast_log::appender::LogAppender;

pub struct UICustomLog;

impl LogAppender for UICustomLog {
    fn do_logs(&mut self, records: &[fast_log::appender::FastLogRecord]) {
        for record in records {
            let data;
            match record.level {
                _ => {
                    data = format!("↑{}\n\n", record.module_path);
                }
            }
            print!("{}", data);
        }
    }
}
