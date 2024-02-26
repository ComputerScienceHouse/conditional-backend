-- Add down migration script here
alter table batch_pull drop constraint batch_pull_pkey;
alter table batch_pull add constraint batch_pull_pkey primary key (uid);
alter table batch_pull drop constraint fk_puller;
alter table batch_pull drop column puller;
alter table batch_pull drop column reason;
alter table batch_pull drop column approved;
