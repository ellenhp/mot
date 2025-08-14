DROP TABLE IF EXISTS poi;
CREATE TABLE poi (
  id SERIAL,
  way_id BIGINT,
  node_id BIGINT,
  tags JSONB,
  geom GEOMETRY
);

-- poi.geom may be any type of geometry except `*LineString` so we can't specify the SRID in the column definition. Change it here.
SELECT UpdateGeometrySRID('poi', 'geom', 4326);

INSERT INTO poi (way_id, tags, geom) SELECT way_id, tags, ST_Centroid(geom) FROM way_poi WHERE ST_Area(geom::GEOGRAPHY, false) < 2000;
INSERT INTO poi (way_id, tags, geom) SELECT way_id, tags, geom FROM way_poi WHERE ST_Area(geom::GEOGRAPHY, false) >= 2000;
INSERT INTO poi (node_id, tags, geom) SELECT node_id, tags, geom FROM node_poi;

CREATE INDEX poi_geom_idx ON poi USING GIST(geom);
CREATE INDEX poi_wid_idx ON poi (way_id);
CREATE INDEX poi_nid_idx ON poi (node_id);

CREATE TABLE IF NOT EXISTS wof_admins (
  id BIGINT,
  geom GEOMETRY(MULTIPOLYGON, 4326),
  admin_level TEXT,
  names TEXT
);

CREATE INDEX IF NOT EXISTS wof_admins_geom ON wof_admins USING GIST(geom);

DROP MATERIALIZED VIEW IF EXISTS edge_intersections;
EXPLAIN ANALYZE CREATE MATERIALIZED VIEW edge_intersections
AS (SELECT DISTINCT
    (ST_Dump(ST_Intersection(l1.geom,l2.geom))).geom AS geom,
    l1.way_id AS way_id,
    l2.way_id AS transition_to_way
    FROM roads AS l1
    INNER JOIN roads AS l2 ON ST_Intersects(l2.geom, l1.geom)
    WHERE l1.way_id <> l2.way_id
) WITH DATA;

DROP MATERIALIZED VIEW IF EXISTS edge_transitions;
EXPLAIN ANALYZE CREATE MATERIALIZED VIEW edge_transitions
AS (SELECT DISTINCT
    ST_Transform(intersections.geom, 4326) AS intersection_geom,
    ST_Length(ST_LineSubstring(roads.geom, 0.0, ST_LineLocatePoint(roads.geom, intersections.geom))::geography) AS distance_along_way,
    intersections.way_id AS way_id,
    intersections.transition_to_way AS transition_to_way,
    roads.tags AS way_tags,
    roads2.tags AS transition_to_way_tags,
    restrictions.tags AS restriction_tags
    FROM roads
    LEFT JOIN edge_intersections AS intersections ON roads.way_id = intersections.way_id
    LEFT JOIN roads as roads2 on intersections.transition_to_way = roads2.way_id
    LEFT JOIN restrictions ON restrictions.from = intersections.way_id AND restrictions.to = intersections.transition_to_way
    WHERE ST_Length(intersections.geom) = 0 -- Fixes: error returned from database: Splitter line has linear intersection with input
);

-- Create the intersections table
DROP TABLE IF EXISTS intersections;
CREATE TABLE intersections (
    way_id BIGINT,
    transition_to_way BIGINT,
    distance_along_way REAL,
    restriction_tags JSONB,
    geom GEOMETRY(Point, 4326)
);

-- Populate the intersections table
INSERT INTO intersections (
    way_id,
    transition_to_way,
    distance_along_way,
    restriction_tags,
    geom
)
SELECT
    way_id,
    transition_to_way,
    distance_along_way,
    restriction_tags,
    intersection_geom
FROM edge_transitions;

CREATE INDEX IF NOT EXISTS idx_intersections_geom ON intersections USING GIST(geom);

