-- Your SQL goes here
create table notes
(
    id    BIGINT PRIMARY KEY NOT NULL,
    title TEXT
);

create table tags
(
    noteId BIGINT NOT NULL,
    tag    TEXT PRIMARY KEY,
    FOREIGN KEY (noteId) REFERENCES Notes (id)
);