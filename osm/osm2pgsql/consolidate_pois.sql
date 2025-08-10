DROP TABLE IF EXISTS poi;

CREATE TABLE poi (
  way_id BIGINT,
  node_id BIGINT,
  tags JSONB,
  geom GEOMETRY
);

SELECT UpdateGeometrySRID('poi', 'geom', 4326);

INSERT INTO poi (way_id, tags, geom) SELECT way_id, tags, geom FROM way_poi;
INSERT INTO poi (node_id, tags, geom) SELECT node_id, tags, geom FROM node_poi;

CREATE INDEX poi_geom_idx ON poi USING GIST(geom);
CREATE INDEX poi_wid_idx ON poi (way_id);
CREATE INDEX poi_nid_idx ON poi (node_id);

WITH subquery AS (
    SELECT
        poi.way_id,
        poi.node_id,
        jsonb_build_object('wof:borough', to_json(array_agg(admins.wof_name))::text) AS newobj
    FROM
        poi
    JOIN
        wof.geojson admins
        ON ST_Intersects(admins.geom, poi.geom)
    WHERE
        admins.wof_name <> ''
        AND admins.wof_placetype = 'borough'
    GROUP BY
        (poi.way_id, poi.node_id)
)
UPDATE poi
SET tags = poi.tags || newobj
FROM subquery
WHERE poi.way_id = subquery.way_id OR poi.node_id = subquery.node_id;

---------

WITH subquery AS (
    SELECT
        poi.way_id,
        poi.node_id,
        jsonb_build_object('wof:campus', to_json(array_agg(admins.wof_name))::text) AS newobj
    FROM
        poi
    JOIN
        wof.geojson admins
        ON ST_Intersects(admins.geom, poi.geom)
    WHERE
        admins.wof_name <> ''
        AND admins.wof_placetype = 'campus'
    GROUP BY
        (poi.way_id, poi.node_id)
)
UPDATE poi
SET tags = poi.tags || newobj
FROM subquery
WHERE poi.way_id = subquery.way_id OR poi.node_id = subquery.node_id;

---------

WITH subquery AS (
    SELECT
        poi.way_id,
        poi.node_id,
        jsonb_build_object('wof:country', to_json(array_agg(admins.wof_name))::text) AS newobj
    FROM
        poi
    JOIN
        wof.geojson admins
        ON ST_Intersects(admins.geom, poi.geom)
    WHERE
        admins.wof_name <> ''
        AND admins.wof_placetype = 'country'
    GROUP BY
        (poi.way_id, poi.node_id)
)
UPDATE poi
SET tags = poi.tags || newobj
FROM subquery
WHERE poi.way_id = subquery.way_id OR poi.node_id = subquery.node_id;

---------

WITH subquery AS (
    SELECT
        poi.way_id,
        poi.node_id,
        jsonb_build_object('wof:county', to_json(array_agg(admins.wof_name))::text) AS newobj
    FROM
        poi
    JOIN
        wof.geojson admins
        ON ST_Intersects(admins.geom, poi.geom)
    WHERE
        admins.wof_name <> ''
        AND admins.wof_placetype = 'county'
    GROUP BY
        (poi.way_id, poi.node_id)
)
UPDATE poi
SET tags = poi.tags || newobj
FROM subquery
WHERE poi.way_id = subquery.way_id OR poi.node_id = subquery.node_id;

---------

WITH subquery AS (
    SELECT
        poi.way_id,
        poi.node_id,
        jsonb_build_object('wof:dependency', to_json(array_agg(admins.wof_name))::text) AS newobj
    FROM
        poi
    JOIN
        wof.geojson admins
        ON ST_Intersects(admins.geom, poi.geom)
    WHERE
        admins.wof_name <> ''
        AND admins.wof_placetype = 'dependency'
    GROUP BY
        (poi.way_id, poi.node_id)
)
UPDATE poi
SET tags = poi.tags || newobj
FROM subquery
WHERE poi.way_id = subquery.way_id OR poi.node_id = subquery.node_id;

---------

WITH subquery AS (
    SELECT
        poi.way_id,
        poi.node_id,
        jsonb_build_object('wof:localadmin', to_json(array_agg(admins.wof_name))::text) AS newobj
    FROM
        poi
    JOIN
        wof.geojson admins
        ON ST_Intersects(admins.geom, poi.geom)
    WHERE
        admins.wof_name <> ''
        AND admins.wof_placetype = 'localadmin'
    GROUP BY
        (poi.way_id, poi.node_id)
)
UPDATE poi
SET tags = poi.tags || newobj
FROM subquery
WHERE poi.way_id = subquery.way_id OR poi.node_id = subquery.node_id;

---------

WITH subquery AS (
    SELECT
        poi.way_id,
        poi.node_id,
        jsonb_build_object('wof:locality', to_json(array_agg(admins.wof_name))::text) AS newobj
    FROM
        poi
    JOIN
        wof.geojson admins
        ON ST_Intersects(admins.geom, poi.geom)
    WHERE
        admins.wof_name <> ''
        AND admins.wof_placetype = 'locality'
    GROUP BY
        (poi.way_id, poi.node_id)
)
UPDATE poi
SET tags = poi.tags || newobj
FROM subquery
WHERE poi.way_id = subquery.way_id OR poi.node_id = subquery.node_id;

---------

WITH subquery AS (
    SELECT
        poi.way_id,
        poi.node_id,
        jsonb_build_object('wof:macrohood', to_json(array_agg(admins.wof_name))::text) AS newobj
    FROM
        poi
    JOIN
        wof.geojson admins
        ON ST_Intersects(admins.geom, poi.geom)
    WHERE
        admins.wof_name <> ''
        AND admins.wof_placetype = 'macrohood'
    GROUP BY
        (poi.way_id, poi.node_id)
)
UPDATE poi
SET tags = poi.tags || newobj
FROM subquery
WHERE poi.way_id = subquery.way_id OR poi.node_id = subquery.node_id;

---------

WITH subquery AS (
    SELECT
        poi.way_id,
        poi.node_id,
        jsonb_build_object('wof:marketarea', to_json(array_agg(admins.wof_name))::text) AS newobj
    FROM
        poi
    JOIN
        wof.geojson admins
        ON ST_Intersects(admins.geom, poi.geom)
    WHERE
        admins.wof_name <> ''
        AND admins.wof_placetype = 'marketarea'
    GROUP BY
        (poi.way_id, poi.node_id)
)
UPDATE poi
SET tags = poi.tags || newobj
FROM subquery
WHERE poi.way_id = subquery.way_id OR poi.node_id = subquery.node_id;

---------

WITH subquery AS (
    SELECT
        poi.way_id,
        poi.node_id,
        jsonb_build_object('wof:microhood', to_json(array_agg(admins.wof_name))::text) AS newobj
    FROM
        poi
    JOIN
        wof.geojson admins
        ON ST_Intersects(admins.geom, poi.geom)
    WHERE
        admins.wof_name <> ''
        AND admins.wof_placetype = 'microhood'
    GROUP BY
        (poi.way_id, poi.node_id)
)
UPDATE poi
SET tags = poi.tags || newobj
FROM subquery
WHERE poi.way_id = subquery.way_id OR poi.node_id = subquery.node_id;

---------

WITH subquery AS (
    SELECT
        poi.way_id,
        poi.node_id,
        jsonb_build_object('wof:neighbourhood', to_json(array_agg(admins.wof_name))::text) AS newobj
    FROM
        poi
    JOIN
        wof.geojson admins
        ON ST_Intersects(admins.geom, poi.geom)
    WHERE
        admins.wof_name <> ''
        AND admins.wof_placetype = 'neighbourhood'
    GROUP BY
        (poi.way_id, poi.node_id)
)
UPDATE poi
SET tags = poi.tags || newobj
FROM subquery
WHERE poi.way_id = subquery.way_id OR poi.node_id = subquery.node_id;

---------

WITH subquery AS (
    SELECT
        poi.way_id,
        poi.node_id,
        jsonb_build_object('wof:region', to_json(array_agg(admins.wof_name))::text) AS newobj
    FROM
        poi
    JOIN
        wof.geojson admins
        ON ST_Intersects(admins.geom, poi.geom)
    WHERE
        admins.wof_name <> ''
        AND admins.wof_placetype = 'region'
    GROUP BY
        (poi.way_id, poi.node_id)
)
UPDATE poi
SET tags = poi.tags || newobj
FROM subquery
WHERE poi.way_id = subquery.way_id OR poi.node_id = subquery.node_id;
