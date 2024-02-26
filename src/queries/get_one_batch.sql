SELECT batch.id as "id!", batch.name AS "name!", batch.creator AS "creator!", bi.conditions AS 
         "conditions!", bi.members AS "members!"
FROM (SELECT cb.bid, cb.conditions, array_agg(cb.uid) AS members
FROM (
SELECT batches.bid
, array_agg(concat(batches.criterion, ' ', batches.comparison, ' ', batches.value)) AS 
         conditions
, batches.mname, batches.uid
FROM (SELECT baid.bid, baid.mname, baid.uid, bc.criterion, bc.comparison, bc.value,
CASE
WHEN baid.bu THEN TRUE
WHEN bc.criterion = 'Packet' AND bc.comparison = 'Greater' THEN evals.packet > bc.value
WHEN bc.criterion = 'Packet' AND bc.comparison = 'Equal' THEN evals.packet = bc.value
WHEN bc.criterion = 'Packet' AND bc.comparison = 'Less' THEN evals.packet < bc.value
WHEN bc.criterion = 'Seminar' AND bc.comparison = 'Greater' THEN evals.ss > bc.value
WHEN bc.criterion = 'Seminar' AND bc.comparison = 'Equal' THEN evals.ss = bc.value
WHEN bc.criterion = 'Seminar' AND bc.comparison = 'Less' THEN evals.ss < bc.value
WHEN bc.criterion = 'Directorship' AND bc.comparison = 'Greater' THEN evals.ds > bc.value
WHEN bc.criterion = 'Directorship' AND bc.comparison = 'Equal' THEN evals.ds = bc.value
WHEN bc.criterion = 'Directorship' AND bc.comparison = 'Less' THEN evals.ds < bc.value
WHEN bc.criterion = 'Missed_HM' AND bc.comparison = 'Greater' THEN evals.hm > bc.value
WHEN bc.criterion = 'Missed_HM' AND bc.comparison = 'Equal' THEN evals.hm = bc.value
WHEN bc.criterion = 'Missed_HM' AND bc.comparison = 'Less' THEN evals.hm < bc.value
ELSE false
END AS cond_passed
FROM (SELECT baid.bid, baid.mname, baid.uid, bool_or(baid.bu) AS bu
FROM (SELECT *
FROM (SELECT bu.batch_id, evals.name, bu.uid, TRUE AS bu
FROM batch_user bu
LEFT JOIN (
SELECT evals.uid, evals.name
FROM (SELECT *
FROM UNNEST($1::varchar[], $2::int4[], $3::int8[], $4::int8[], $5::int8[], $6::int8[])) 
AS evals("name", uid, ss, ds, hm, packet)) evals
ON bu.uid = evals.uid) AS frosh_info
UNION (
SELECT batch.id, evals.name, evals.uid, 
         FALSE AS bu
FROM batch,
	(SELECT * FROM UNNEST($1::varchar[], $2::int4[], $3::int8[], $4::int8[], $5::int8[], 
         $6::int8[])) AS evals("name", uid, ss, ds, hm, packet)
where batch.id = $7)) AS baid(bid, mname, uid, bu)
GROUP BY baid.bid, baid.mname, baid.uid) AS baid
LEFT JOIN batch_condition bc ON bc.batch_id=baid.bid
LEFT JOIN (
SELECT evals.uid, evals.ss, evals.ds, evals.hm, evals.packet
FROM (SELECT *
FROM UNNEST($1::varchar[], $2::int4[], $3::int8[], $4::int8[], $5::int8[], $6::int8[])) 
AS evals("name", uid, ss, ds, hm, packet)
) evals ON evals.uid=baid.uid
WHERE NOT EXISTS (SELECT 1 FROM batch_pull bp WHERE bp.approved AND bp.uid=baid.uid)) AS 
         batches
GROUP BY batches.bid, batches.mname, batches.uid
HAVING bool_and(batches.cond_passed)) AS cb
GROUP BY cb.bid, cb.conditions) AS bi --thats gay
LEFT JOIN batch ON bi.bid=batch.id
