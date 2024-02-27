select
m.uid, 
m.name,
m.seminars,
m.directorships,
count(ha.attendance_status)
    filter(where ha.attendance_status = 'Absent') as missed_hms
from (
    select
        u.name,
        u.id as uid,
        count(om.approved)
            filter(where om.meeting_type = 'Seminar') as seminars,
        count(om.approved)
            filter(where om.meeting_type = 'Directorship') as directorships
    from "user" u
    left join om_attendance oma on u.id = oma.uid
    left join other_meeting om on oma.om_id = om.id
    left join intro_eval_block ieb on ieb.id = (select max(id) from intro_eval_block)
    where
        not is_intro
        and u.csh_username in (select UNNEST($1::varchar[]))
        and u.name in (select UNNEST($2::varchar[]))
        and om.datetime between ieb.start_date and ieb.end_date
    group by
        u.id, u.name
    ) as m
    left join hm_attendance ha on m.uid = ha.uid
    group by
        m.uid,
        m.name,
        m.seminars,
        m.directorships
