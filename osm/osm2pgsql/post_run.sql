DROP TABLE IF EXISTS poi;

CREATE TABLE poi (
  way_id BIGINT,
  node_id BIGINT,
  tags JSONB,
  geom GEOMETRY
);

-- poi.geom may be any type of geometry except `*LineString` so we can't specify the SRID in the column definition. Change it here.
SELECT UpdateGeometrySRID('poi', 'geom', 4326);

INSERT INTO poi (way_id, tags, geom) SELECT way_id, tags, geom FROM way_poi;
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

DROP TABLE IF EXISTS tiles;
CREATE TABLE tiles (
  idx INTEGER,
  x INTEGER,
  y INTEGER,
  z INTEGER,
  geom GEOMETRY(POLYGON, 4326)
);

WITH tile_coords AS
(
  select generate_series as idx, floor(generate_series/4096)::integer as x, (generate_series%4096)::integer as y from generate_series(0, 16777215)
)
INSERT INTO tiles (
  idx, x, y, z, geom
)
SELECT
  tile_coords.idx as idx,
  tile_coords.x as x,
  tile_coords.y as y,
  12 as z,
  ST_Transform(ST_TileEnvelope(12, tile_coords.x, tile_coords.y), 4326) as geom
FROM tile_coords;

CREATE INDEX IF NOT EXISTS tiles_geom ON tiles USING GIST(geom);

CREATE OR REPLACE VIEW edge_intersections
AS (SELECT DISTINCT
    (ST_Dump(ST_Intersection(l1.geom,l2.geom))).geom AS geom,
    l1.way_id AS way_id,
    l2.way_id AS transition_to_way
    FROM lines AS l1
    INNER JOIN lines AS l2 ON ST_Intersects(l2.geom, l1.geom)
    WHERE l1.way_id <> l2.way_id
);

CREATE OR REPLACE VIEW edge_transitions
AS (SELECT DISTINCT
    ST_Transform(intersections.geom, 4326) AS intersection_geom,
    ST_Length(ST_LineSubstring(lines.geom, 0.0, ST_LineLocatePoint(lines.geom, intersections.geom))::geography) AS distance_along_way,
    ST_Length(ST_LineSubstring(lines2.geom, 0.0, ST_LineLocatePoint(lines2.geom, intersections.geom))::geography) AS transition_to_distance_along_way,
    intersections.way_id AS way_id,
    intersections.transition_to_way AS transition_to_way,
    lines.tags AS way_tags,
    lines2.tags AS transition_to_way_tags,
    restrictions.tags AS restriction_tags
    FROM lines
    LEFT JOIN edge_intersections AS intersections ON lines.way_id = intersections.way_id
    LEFT JOIN lines as lines2 on intersections.transition_to_way = lines2.way_id
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
    restriction_tags,
    intersection_geom
FROM edge_transitions;

CREATE INDEX IF NOT EXISTS idx_intersections_geom ON intersections USING GIST(geom);

