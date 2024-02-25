-- Add up migration script here
alter table coop add year int4 not null default 0;
update coop set year = (select 
  case when (select date_part('month', "date") from coop c where c.id = coop.id) > 5 then 
    date_part('year', "date") 
  else date_part('year', "date") - 1 
end 
from coop c where c.id = coop.id);
alter table coop drop column date;
ALTER TABLE coop DROP CONSTRAINT coop_pkey;
ALTER TABLE coop ADD CONSTRAINT coop_pk PRIMARY KEY (uid,"year", semester);
alter table coop drop column id;
