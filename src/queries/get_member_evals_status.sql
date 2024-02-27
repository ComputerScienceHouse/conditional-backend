select
    h.uid, 
    h.name,
    h.seminars,
    h.directorships,
    h.missed_hms,
    count(mp.status)
        filter(where mp.status = 'Passed') as major_projects
from (
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
        where
            not is_intro and om.datetime > $1
            and u.csh_username in (select UNNEST($2::varchar[]))
            and u.name in (select UNNEST($3::varchar[]))
        group by
            u.id, u.name
        ) as m
        left join hm_attendance ha on m.uid = ha.uid
        group by
            m.uid,
            m.name,
            m.seminars,
            m.directorships
    ) as h
    left join major_project mp on h.uid = mp.uid
group by
    h.name,
    h.uid,
    h.seminars,
    h.directorships,
    h.missed_hms
