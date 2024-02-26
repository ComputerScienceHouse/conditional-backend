-- Add up migration script here
ALTER TABLE intro_eval_data DROP CONSTRAINT intro_eval_data_pkey;
ALTER TABLE intro_eval_data ADD CONSTRAINT intro_eval_data_pkey PRIMARY KEY (uid, eval_block_id);
alter table intro_eval_data drop column id;
