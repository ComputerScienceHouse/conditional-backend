-- Add up migration script here
create table freshman_rooms(
  uid int4 primary key not null,
  room int4 not null,
  constraint fk_user
    foreign key(uid)
      references "user"(id)
);
