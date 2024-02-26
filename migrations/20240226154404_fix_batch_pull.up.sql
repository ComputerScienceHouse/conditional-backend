-- Add up migration script here
alter table batch_pull add puller int4 not null;
alter table batch_pull add reason varchar not null;
alter table batch_pull add approved boolean not null;
alter table batch_pull add constraint fk_puller foreign key(puller) references "user"(id);
alter table batch_pull drop constraint batch_pull_pkey;
alter table batch_pull add constraint batch_pull_pkey primary key (uid, puller)
