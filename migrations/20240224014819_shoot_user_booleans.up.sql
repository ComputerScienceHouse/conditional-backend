-- Add up migration script here
ALTER TABLE "user" DROP COLUMN is_csh;
ALTER TABLE "user" DROP COLUMN is_intro;
