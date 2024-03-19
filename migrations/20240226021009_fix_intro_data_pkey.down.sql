-- Add down migration script here
ALTER TABLE intro_eval_data DROP CONSTRAINT intro_eval_data_pkey;
ALTER TABLE intro_eval_data ADD id int4 GENERATED ALWAYS AS IDENTITY( INCREMENT BY 1 MINVALUE 1 MAXVALUE 2147483647 START 1 CACHE 1 NO CYCLE) NOT NULL;
ALTER TABLE intro_eval_data ADD CONSTRAINT intro_eval_data_pkey PRIMARY KEY (id);
