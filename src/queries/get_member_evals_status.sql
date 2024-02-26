
select
    s.name,
    s.username,
    s.uid,
    s.seminars,
    s.directorships,
    s.missed_hms,
    s.major_projects
from
    (
    select
        u.name,
        u.rit_username as username,
        u.id as uid,
        count(om.approved)
       filter(
    where
        om.meeting_type = 'Seminar') as seminars,
        count(om.approved)
        filter(
    where
        om.meeting_type = 'Directorship') as directorships,
        count(ha.attendance_status)
        filter(
        where ha.attendance_status = 'Absent') as missed_hms,
        count(mp.status)
        filter(
        where mp.status = 'Passed') as major_projects
    from
        "user" u
    left join om_attendance oma on
        u.id = oma.uid
    left join other_meeting om on
        oma.om_id = om.id
    left join hm_attendance ha on
        u.id = ha.uid
    left join major_project mp on
        u.id = mp.uid
    where
        not is_intro and om.datetime > $1
        and u.csh_username in (select UNNEST($2::varchar[]))
        and u.name in (select UNNEST($3::varchar[]))
    group by
        u.rit_username,
        u.id) as s
group by
    s.name,
    s.username,
    s.uid,
    s.seminars,
    s.directorships,
    s.missed_hms,
    s.major_projects
