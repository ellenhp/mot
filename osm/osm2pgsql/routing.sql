
-- CLUSTER lines USING lines_geom_idx;

-- CREATE OR REPLACE VIEW edge_intersections
-- AS (SELECT DISTINCT
--     (ST_Dump(ST_Intersection(l1.geom,l2.geom))).geom AS geom,
--     l1.way_id AS way_id,
--     l2.way_id AS transition_to_way
--     FROM lines AS l1
--     INNER JOIN lines AS l2 ON ST_Intersects(l2.geom, l1.geom)
--     WHERE l1.way_id <> l2.way_id
-- );

-- CREATE MATERIALIZED VIEW IF NOT EXISTS edge_transitions
-- AS (SELECT DISTINCT
--     ST_Transform(intersections.geom, 4326) AS intersection_geom,
--     ST_Length(ST_LineSubstring(lines.geom, 0.0, ST_LineLocatePoint(lines.geom, intersections.geom))::geography) AS distance_along_way,
--     ST_Length(ST_LineSubstring(lines2.geom, 0.0, ST_LineLocatePoint(lines2.geom, intersections.geom))::geography) AS transition_to_distance_along_way,
--     intersections.way_id AS way_id,
--     intersections.transition_to_way AS transition_to_way,
--     lines.tags AS way_tags,
--     lines2.tags AS transition_to_way_tags,
--     restrictions.tags AS restriction_tags
--     FROM lines
--     LEFT JOIN edge_intersections AS intersections ON lines.way_id = intersections.way_id
--     LEFT JOIN lines as lines2 on intersections.transition_to_way = lines2.way_id
--     LEFT JOIN restrictions ON restrictions.from = intersections.way_id AND restrictions.to = intersections.transition_to_way
--     WHERE ST_Length(intersections.geom) = 0 -- Fixes: error returned from database: Splitter line has linear intersection with input
-- )
-- WITH DATA;

-- CREATE UNIQUE INDEX IF NOT EXISTS edge_transitions_unique ON edge_transitions (
--     intersection_geom,
--     distance_along_way,
--     transition_to_distance_along_way,
--     way_id,
--     transition_to_way,
--     transition_to_way_tags,
--     restriction_tags
-- );

-- CREATE INDEX IF NOT EXISTS edge_transitions_by_way ON edge_transitions (
--     way_id
-- );

-- CREATE INDEX IF NOT EXISTS edge_transitions_by_transition_to_way ON edge_transitions (
--     transition_to_way
-- );

-- CREATE MATERIALIZED VIEW IF NOT EXISTS edge_metadata
-- AS (SELECT DISTINCT
--     l1.way_id AS way_id,
--     l1.tags AS tags
--     FROM lines AS l1
-- )
-- WITH DATA;

-- CREATE UNIQUE INDEX IF NOT EXISTS edge_metadata_unique ON edge_metadata (
--     way_id,
--     tags
-- );
