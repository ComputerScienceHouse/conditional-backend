SELECT us.username,
    us.name,
    (LEAST(count(sm.packet_id), 10) + upper_signs) AS signatures,
    us.max_upper + 10 AS max_signatures
FROM(SELECT fm.name,
            p.id,
            p.freshman_username AS username,
            count(su.signed) FILTER(WHERE su.signed) AS upper_signs,
            count(su.packet_id) AS max_upper
     FROM freshman fm
     LEFT JOIN packet p ON
         fm.rit_username = p.freshman_username
     LEFT JOIN signature_upper su ON
         p.id = su.packet_id
     WHERE p.freshman_username IS NOT NULL
     GROUP BY p.id, fm.name) AS us
 LEFT JOIN signature_misc sm ON
     us.id = sm.packet_id
 GROUP BY us.username, upper_signs, us.id, us.max_upper, us.name
