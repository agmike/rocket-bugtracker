DROP TRIGGER log_change ON actions;
DROP TRIGGER log_change ON users;
DROP TRIGGER log_change ON tags;
DROP TRIGGER log_change ON issues;
DROP TRIGGER log_change ON issue_tags;

DROP FUNCTION log_change;
DROP TABLE log;