-- Add up migration script here
ALTER TABLE public."user" ADD is_intro boolean DEFAULT true NOT NULL;
