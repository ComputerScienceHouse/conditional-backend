-- Add down migration script here
alter table coop add date date not null default CURRENT_DATE;
update coop set date = (select date((year*10000+601)::TEXT));
alter table coop drop column year;
ALTER TABLE coop DROP CONSTRAINT coop_pk;
ALTER TABLE coop ADD id int4 GENERATED ALWAYS AS IDENTITY( INCREMENT BY 1 MINVALUE 1 MAXVALUE 2147483647 START 1 CACHE 1 NO CYCLE) NOT NULL;
ALTER TABLE coop ADD CONSTRAINT coop_pkey PRIMARY KEY id;
