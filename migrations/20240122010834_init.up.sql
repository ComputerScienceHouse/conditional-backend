-- Enums
-- {{{
CREATE TYPE public."batch_comparison_enum" AS ENUM (
	'Greater',
	'Equal',
	'Less');

CREATE TYPE public."batch_criterion_enum" AS ENUM (
	'Seminar',
	'Directorship',
	'Packet',
	'Missed_HM');

CREATE TYPE public."conditional_status_enum" AS ENUM (
	'Passed',
	'Pending',
	'Failed');

CREATE TYPE public."eval_status_enum" AS ENUM (
	'Pending',
	'Passed',
	'Failed');

CREATE TYPE public."hm_attendance_status_enum" AS ENUM (
	'Attended',
	'Absent',
	'Excused');

CREATE TYPE public."major_project_status_enum" AS ENUM (
	'Pending',
	'Passed',
	'Failed');

CREATE TYPE public."meeting_type_enum" AS ENUM (
	'Seminar',
	'Directorship');

CREATE TYPE public."semester_enum" AS ENUM (
	'Fall',
	'Spring',
	'Summer');
-- }}}

-- Tables

-- {{{
-- user
CREATE TABLE public."user" (
	id int4 PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
	"name" varchar NOT NULL,
	uuid varchar NOT NULL,
  rit_username varchar NOT NULL,
  csh_username varchar NULL,
	is_csh bool NOT NULL,
	is_intro bool NOT NULL
);
CREATE INDEX user_uuid_idx ON public."user" USING btree (uuid);

-- intro eval block
CREATE TABLE public.intro_eval_block (
	id int4 PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
	start_date date NOT NULL,
	end_date date NOT NULL
);

-- house meeting
CREATE TABLE public.house_meeting (
	id int4 PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
	"date" date NOT NULL
);

-- other meeting
CREATE TABLE public.other_meeting (
	id int4 PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
	"date" date NOT NULL,
	"name" varchar NOT NULL,
	meeting_type public."meeting_type_enum" NOT NULL,
	approved bool NOT NULL
);
-- }}}

-- {{{
-- other meeting attendance
CREATE TABLE om_attendance (
	uid int4 NOT NULL REFERENCES "user"(id) ON DELETE CASCADE,
	om_id int4 NOT NULL REFERENCES other_meeting(id) ON DELETE CASCADE,
	CONSTRAINT om_attendance_pkey PRIMARY KEY (uid, om_id)
);

-- house meeting attendance
CREATE TABLE hm_attendance (
	uid int4 NOT NULL REFERENCES "user"(id) ON DELETE CASCADE,
	house_meeting_id int4 NOT NULL REFERENCES house_meeting(id) ON DELETE CASCADE,
	attendance_status public."hm_attendance_status_enum" NOT NULL,
	excuse varchar NULL
);

-- intro eval data
CREATE TABLE intro_eval_data (
	id int4 PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
	uid int4 NOT NULL REFERENCES "user"(id) ON DELETE CASCADE,
	eval_block_id int4 NOT NULL REFERENCES intro_eval_block(id) ON DELETE CASCADE,
	social_events varchar NOT NULL,
	other_comments varchar NOT NULL,
	status public."eval_status_enum" NOT NULL
);

-- batch
CREATE TABLE batch (
	id int4 PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
	"name" varchar(64) NOT NULL,
	creator int4 NOT NULL REFERENCES "user"(id) ON DELETE CASCADE,
	approved bool NOT NULL
);

-- batch pull
CREATE TABLE batch_pull (
	uid int4 NOT NULL REFERENCES "user"(id) ON DELETE CASCADE,
	CONSTRAINT batch_pull_pkey PRIMARY KEY (uid)
);

-- conditional
CREATE TABLE conditional (
	id int4 PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
	uid int4 NOT NULL REFERENCES "user"(id) ON DELETE CASCADE,
	description text NOT NULL,
	start_date date NOT NULL,
	due_date date NOT NULL,
	status public."conditional_status_enum" NOT NULL
);

-- coop
CREATE TABLE coop (
	id int4 PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
	uid int4 NOT NULL REFERENCES "user"(id) ON DELETE CASCADE,
	"date" date NOT NULL,
	semester public."semester_enum" NOT NULL
);

-- housing queue
CREATE TABLE housing_queue (
	uid int4 NOT NULL REFERENCES "user"(id) ON DELETE CASCADE,
	datetime_added timestamp NOT NULL,
	CONSTRAINT housing_queue_pkey PRIMARY KEY (uid)
);

-- major project
CREATE TABLE major_project (
	id int4 PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
	uid int4 NOT NULL REFERENCES "user"(id) ON DELETE CASCADE,
	"name" varchar(80) NOT NULL,
	description text NOT NULL,
	"date" date NOT NULL,
	status public."major_project_status_enum" NOT NULL
);


-- member eval data
CREATE TABLE member_eval_data (
	uid int4 NOT NULL REFERENCES "user"(id) ON DELETE CASCADE,
	"year" int4 NOT NULL,
	status public."eval_status_enum" NOT NULL
);
-- }}}

-- {{{
-- batch user
CREATE TABLE batch_user (
	uid int4 NOT NULL REFERENCES "user"(id) ON DELETE CASCADE,
	batch_id int4 NOT NULL REFERENCES batch(id ON DELETE CASCADE)
);

-- batch condition
CREATE TABLE batch_condition (
	id int4 PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
	batch_id int4 NOT NULL REFERENCES batch(id) ON DELETE CASCADE,
	value int4 NOT NULL,
	criterion public."batch_criterion_enum" NOT NULL,
	comparison public."batch_comparison_enum" NOT NULL
);
-- }}}

-- insert the intro eval block
INSERT INTO intro_eval_block (start_date, end_date) VALUES('2023-08-20', '2023-10-15');
