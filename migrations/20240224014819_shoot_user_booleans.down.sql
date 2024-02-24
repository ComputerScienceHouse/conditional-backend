-- Add down migration script here
ALTER TABLE "user" ADD COLUMN is_csh boolean GENERATED ALWAYS AS (ipa_unique_id IS NOT NULL), is_intro boolean NOT NULL DEFAULT true
