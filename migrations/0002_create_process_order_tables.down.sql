-- Migration: 0002_create_process_order_tables  (down)
DROP INDEX IF EXISTS idx_po_executions_confirmed_at;
DROP INDEX IF EXISTS idx_po_executions_order_id;
DROP INDEX IF EXISTS idx_process_orders_status;
DROP INDEX IF EXISTS idx_process_orders_workflow_id;
DROP TABLE IF EXISTS process_order_executions;
DROP TABLE IF EXISTS process_orders;
