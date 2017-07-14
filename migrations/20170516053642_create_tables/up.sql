CREATE TABLE users (
  id SERIAL PRIMARY KEY,
  name VARCHAR NOT NULL,
  email VARCHAR NOT NULL,
  password VARCHAR NOT NULL
);

INSERT INTO users VALUES (0, 'root', 'root', 'root');

CREATE TABLE issues (
  id SERIAL PRIMARY KEY,
  title VARCHAR NOT NULL,
  description TEXT NOT NULL
);

INSERT INTO issues VALUES (0, 'global', 'Глобальные действия отображаются здесь');

CREATE TABLE tags (
  id SERIAL PRIMARY KEY,
  name VARCHAR NOT NULL
);

INSERT INTO tags VALUES (0, 'closed');

CREATE TABLE actions (
  id SERIAL PRIMARY KEY,
  user_id INTEGER NOT NULL REFERENCES users(id),
  issue_id INTEGER NOT NULL REFERENCES issues(id),
  time TIMESTAMP WITHOUT TIME ZONE NOT NULL,
  comment TEXT,
  add_tag INTEGER,
  remove_tag INTEGER,
  create_issue INTEGER,
  create_user INTEGER
 );

 CREATE TABLE issue_tags (
   id SERIAL PRIMARY KEY,
   issue_id INTEGER NOT NULL REFERENCES issues(id),
   tag_id INTEGER NOT NULL REFERENCES tags(id)
 );