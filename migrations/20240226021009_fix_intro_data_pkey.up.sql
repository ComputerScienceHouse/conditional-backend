-- Add up migration script here
CREATE TABLE intro_eval_data_temp (LIKE intro_eval_data);
INSERT INTO intro_eval_data_temp (id, uid, eval_block_id, social_events, other_comments, status)
SELECT DISTINCT ON (uid, eval_block_id) id, uid, eval_block_id, social_events, other_comments, status FROM intro_eval_data;
DROP TABLE intro_eval_data;
ALTER TABLE intro_eval_data_temp RENAME TO intro_eval_data;
ALTER TABLE intro_eval_data ADD CONSTRAINT intro_eval_data_pkey PRIMARY KEY (uid, eval_block_id);
ALTER TABLE intro_eval_data DROP COLUMN id;
