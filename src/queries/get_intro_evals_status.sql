select
    s.name,
    s.username,
    s.uid,
    s.seminars,
    s.directorships,
    packet.signatures,
    packet.max_signatures,
    count(ha.attendance_status)
    filter(where ha.attendance_status = 'Absent') as missed_hms
    from
        (
        select
            u.name,
            u.rit_username as username,
            u.id as uid,
            count(om.approved)
        filter(where om.meeting_type = 'Seminar') as seminars,
            count(om.approved)
        filter(where om.meeting_type = 'Directorship') as directorships
        from
            "user" u
        left join om_attendance oma on u.id = oma.uid
        left join other_meeting om on oma.om_id = om.id
        left join intro_eval_data ied on u.id = ied.uid
        left join intro_eval_block ieb on ieb.id = ied.eval_block_id
        where
            ied.eval_block_id = $5 and u.is_intro and om.datetime between ieb.start_date and ieb.end_date
        group by
            u.rit_username,
            u.id) as s
    left join hm_attendance ha on
        s.uid = ha.uid
    left join unnest($1::varchar[],
        $2::varchar[],
        $3::int8[],
        $4::int8[]) as
        packet(username,
        name,
        signatures,
        max_signatures) on
        packet.username = s.username
    where
        packet.name is not null
        and s.seminars is not null
        and s.directorships is not null
        and packet.signatures is not null
        and packet.max_signatures is not null
    group by
        s.name,
        s.username,
        s.uid,
        s.seminars,
        s.directorships,
        packet.signatures,
        packet.max_signatures
