CREATE TABLE public.settings (
	id int4 PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
  current_eval_block int4 NOT NULL REFERENCES intro_eval_block(id) ON DELETE RESTRICT
);

INSERT INTO settings (current_eval_block) VALUES (1);
