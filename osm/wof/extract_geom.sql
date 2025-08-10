BEGIN;
ALTER TABLE wof.geojson ADD COLUMN props JSON;
ALTER TABLE wof.geojson ALTER COLUMN props SET DATA TYPE JSON USING (body::json->>'properties')::json;
ALTER TABLE wof.geojson ADD COLUMN wof_name TEXT;
ALTER TABLE wof.geojson ALTER COLUMN wof_name SET DATA TYPE TEXT USING (props::json->>'wof:name');
ALTER TABLE wof.geojson ADD COLUMN wof_placetype TEXT;
ALTER TABLE wof.geojson ALTER COLUMN wof_placetype SET DATA TYPE TEXT USING (props::json->>'wof:placetype');
ALTER TABLE wof.geojson ALTER COLUMN body TYPE GEOMETRY USING ST_SetSRID(ST_GeomFromGeoJSON(body::json->>'geometry'), 4326);
ALTER TABLE wof.geojson RENAME COLUMN body TO geom;
COMMIT;

SELECT UpdateGeometrySRID('wof', 'geojson','geom', 4326);

CREATE INDEX geojson_geom_idx
  ON wof.geojson
  USING GIST (geom);

CREATE INDEX geojson_placetype_idx
  ON wof.geojson (wof_placetype);

