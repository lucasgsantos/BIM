-- Migration: 0002_create_process_order_tables  (up)

CREATE TABLE IF NOT EXISTS process_orders (
    id              UUID        PRIMARY KEY DEFAULT uuid_generate_v4(),
    order_number    TEXT        NOT NULL UNIQUE,
    description     TEXT        NOT NULL DEFAULT '',
    workflow_id     UUID        REFERENCES workflows(id) ON DELETE SET NULL,
    workflow_name   TEXT        NOT NULL DEFAULT '',
    status          TEXT        NOT NULL DEFAULT 'pending'
                        CHECK (status IN ('pending', 'in_progress', 'completed', 'cancelled')),
    assigned_to     TEXT        NOT NULL DEFAULT '',
    scheduled_date  TEXT        NOT NULL DEFAULT '',
    current_step    INTEGER     NOT NULL DEFAULT 0,
    total_steps     INTEGER     NOT NULL DEFAULT 0,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS process_order_executions (
    id                  UUID        PRIMARY KEY DEFAULT uuid_generate_v4(),
    process_order_id    UUID        NOT NULL
                            REFERENCES process_orders(id) ON DELETE CASCADE,
    node_canvas_id      TEXT        NOT NULL,
    node_type           TEXT        NOT NULL,
    node_label          TEXT        NOT NULL,
    step_number         INTEGER     NOT NULL,
    confirmed_by        TEXT        NOT NULL DEFAULT '',
    notes               TEXT        NOT NULL DEFAULT '',
    confirmed_at        TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_process_orders_workflow_id
    ON process_orders(workflow_id);
CREATE INDEX IF NOT EXISTS idx_process_orders_status
    ON process_orders(status);
CREATE INDEX IF NOT EXISTS idx_po_executions_order_id
    ON process_order_executions(process_order_id);
CREATE INDEX IF NOT EXISTS idx_po_executions_confirmed_at
    ON process_order_executions(confirmed_at);
