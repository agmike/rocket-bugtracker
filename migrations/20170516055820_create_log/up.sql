CREATE TABLE log (
  id SERIAL PRIMARY KEY,
  time TIMESTAMP WITHOUT TIME ZONE NOT NULL,
  action VARCHAR NOT NULL,
  table_name VARCHAR NOT NULL,
  old_val TEXT,
  new_val TEXT,
  query TEXT
);

CREATE OR REPLACE FUNCTION log_change() RETURNS TRIGGER AS $body$
DECLARE
    v_old_data TEXT;
    v_new_data TEXT;
BEGIN
    IF (TG_OP = 'UPDATE') THEN
        v_old_data := ROW(OLD.*);
        v_new_data := ROW(NEW.*);
        INSERT INTO log (time, action, old_val, new_val, table_name, query)
        VALUES (now(), TG_OP, v_old_data,v_new_data, TG_TABLE_NAME, current_query());
        RETURN NEW;
    ELSIF (TG_OP = 'DELETE') THEN
        v_old_data := ROW(OLD.*);
        INSERT INTO log (time, action, old_val, new_val, table_name, query)
        VALUES (now(), TG_OP, v_old_data,v_new_data, TG_TABLE_NAME, current_query());
        RETURN OLD;
    ELSIF (TG_OP = 'INSERT') THEN
        v_new_data := ROW(NEW.*);
        INSERT INTO log (time, action, old_val, new_val, table_name, query)
        VALUES (now(), TG_OP, v_old_data,v_new_data, TG_TABLE_NAME, current_query());
        RETURN NEW;
    END IF;
END;
$body$
LANGUAGE plpgsql;

CREATE TRIGGER log_change AFTER INSERT OR UPDATE OR DELETE ON actions FOR EACH ROW EXECUTE PROCEDURE log_change();
CREATE TRIGGER log_change AFTER INSERT OR UPDATE OR DELETE ON users FOR EACH ROW EXECUTE PROCEDURE log_change();
CREATE TRIGGER log_change AFTER INSERT OR UPDATE OR DELETE ON tags FOR EACH ROW EXECUTE PROCEDURE log_change();
CREATE TRIGGER log_change AFTER INSERT OR UPDATE OR DELETE ON issues FOR EACH ROW EXECUTE PROCEDURE log_change();
CREATE TRIGGER log_change AFTER INSERT OR UPDATE OR DELETE ON issue_tags FOR EACH ROW EXECUTE PROCEDURE log_change();