-- Add up migration script here
CREATE INDEX idx_res_order_1_thread ON responses(res_order, thread_id);
