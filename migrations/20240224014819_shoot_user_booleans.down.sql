-- Add down migration script here
ALTER TABLE "user" ADD COLUMN is_csh boolean GENERATED ALWAYS AS ("user".ipa_unique_id IS NOT NULL) STORED;
ALTER TABLE "user" ADD COLUMN is_intro boolean NOT NULL DEFAULT true;
