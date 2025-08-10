-- Set this to the projection you want to use
local srid = 4326

local tables = {}

tables.roads = osm2pgsql.define_way_table('roads', {
    { column = 'tags', type = 'jsonb' },
    { column = 'rel_ids', sql_type = 'int8[]' }, -- array with integers (for relation IDs)})
    { column = 'geom', type = 'linestring', projection = srid, not_null = true },
})

tables.way_poi = osm2pgsql.define_way_table('way_poi', {
    { column = 'tags', type = 'jsonb' },
    { column = 'geom', type = 'polygon', projection = srid, not_null = true },
})

tables.node_poi = osm2pgsql.define_node_table('node_poi', {
    { column = 'tags', type = 'jsonb' },
    { column = 'geom', type = 'point', projection = srid, not_null = true },
})

tables.restrictions = osm2pgsql.define_table({
    name = 'pois',
    ids = { type = 'relation', id_column = 'relation_id' },
    columns = {
        { column = 'ref', type = 'text' },
        { column = 'from', type = 'int8' },
        { column = 'to', type = 'int8' },
        { column = 'tags', type = 'jsonb' },
        { column = 'members', type = 'jsonb' },
    },
    indexes = {
        { column = {'from', 'to'}, method = 'btree' },
    }
})


tables.restrictions = osm2pgsql.define_table({
    name = 'restrictions',
    ids = { type = 'relation', id_column = 'relation_id' },
    columns = {
        { column = 'ref', type = 'text' },
        { column = 'from', type = 'int8' },
        { column = 'to', type = 'int8' },
        { column = 'tags', type = 'jsonb' },
        { column = 'members', type = 'jsonb' },
    },
    indexes = {
        { column = {'from', 'to'}, method = 'btree' },
    }
})


-- These tag keys are generally regarded as useless for most rendering. Most
-- of them are from imports or intended as internal information for mappers.
--
-- If a key ends in '*' it will match all keys with the specified prefix.
--
-- If you want some of these keys, perhaps for a debugging layer, just
-- delete the corresponding lines.
local delete_keys = {

    -- "mapper" keys
    'attribution',
    'comment',
    'created_by',
    'fixme',
    'note',
    'note:*',
    'odbl',
    'odbl:note',
    'source',
    'source:*',
    'source_ref',

    -- "import" keys

    -- Corine Land Cover (CLC) (Europe)
    'CLC:*',

    -- Geobase (CA)
    'geobase:*',
    -- CanVec (CA)
    'canvec:*',

    -- osak (DK)
    'osak:*',
    -- kms (DK)
    'kms:*',

    -- ngbe (ES)
    -- See also note:es and source:file above
    'ngbe:*',

    -- Friuli Venezia Giulia (IT)
    'it:fvg:*',

    -- KSJ2 (JA)
    -- See also note:ja and source_ref above
    'KSJ2:*',
    -- Yahoo/ALPS (JA)
    'yh:*',

    -- LINZ (NZ)
    'LINZ2OSM:*',
    'linz2osm:*',
    'LINZ:*',
    'ref:linz:*',

    -- WroclawGIS (PL)
    'WroclawGIS:*',
    -- Naptan (UK)
    'naptan:*',

    -- TIGER (US)
    'tiger:*',
    -- GNIS (US)
    'gnis:*',
    -- National Hydrography Dataset (US)
    'NHD:*',
    'nhd:*',
    -- mvdgis (Montevideo, UY)
    'mvdgis:*',

    -- EUROSHA (Various countries)
    'project:eurosha_2012',

    -- UrbIS (Brussels, BE)
    'ref:UrbIS',

    -- NHN (CA)
    'accuracy:meters',
    'sub_sea:type',
    'waterway:type',
    -- StatsCan (CA)
    'statscan:rbuid',

    -- RUIAN (CZ)
    'ref:ruian:addr',
    'ref:ruian',
    'building:ruian:type',
    -- DIBAVOD (CZ)
    'dibavod:id',
    -- UIR-ADR (CZ)
    'uir_adr:ADRESA_KOD',

    -- GST (DK)
    'gst:feat_id',

    -- Maa-amet (EE)
    'maaamet:ETAK',
    -- FANTOIR (FR)
    'ref:FR:FANTOIR',

    -- 3dshapes (NL)
    '3dshapes:ggmodelk',
    -- AND (NL)
    'AND_nosr_r',

    -- OPPDATERIN (NO)
    'OPPDATERIN',
    -- Various imports (PL)
    'addr:city:simc',
    'addr:street:sym_ul',
    'building:usage:pl',
    'building:use:pl',
    -- TERYT (PL)
    'teryt:simc',

    -- RABA (SK)
    'raba:id',
    -- DCGIS (Washington DC, US)
    'dcgis:gis_id',
    -- Building Identification Number (New York, US)
    'nycdoitt:bin',
    -- Chicago Building Inport (US)
    'chicago:building_id',
    -- Louisville, Kentucky/Building Outlines Import (US)
    'lojic:bgnum',
    -- MassGIS (Massachusetts, US)
    'massgis:way_id',
    -- Los Angeles County building ID (US)
    'lacounty:*',
    -- Address import from Bundesamt für Eich- und Vermessungswesen (AT)
    'at_bev:addr_date',

    -- misc
    'import',
    'import_uuid',
    'OBJTYPE',
    'SK53_bulk:load',
    'mml:class',

    -- custom
    'mapillary'
}

local n2as = {}
local w2as = {}
local w2r = {}

local function unique_array(array)
    local result = {}

    local last = nil
    for _, v in ipairs(array) do
        if v ~= last then
            result[#result + 1] = v
            last = v
        end
    end

    return result
end

local function is_restriction(tags)
    return tags.type == 'restriction'
end

local function is_associated_street(tags)
    return tags.type == 'associatedStreet' or tags.type == 'street'
end

-- The osm2pgsql.make_clean_tags_func() function takes the list of keys
-- and key prefixes defined above and returns a function that can be used
-- to clean those tags out of a Lua table. The clean_tags function will
-- return true if it removed all tags from the table.
local clean_tags = osm2pgsql.make_clean_tags_func(delete_keys)

local function is_roadway(tags)
    return tags.highway
end

local function is_poi(tags)
    return (tags['addr:street'] and tags['addr:housenumber']) or tags.amenity or tags.shop or tags.leisure or tags.office or tags.tourism or tags.natural or tags.healthcare or tags.emergency or tags.craft
end

function osm2pgsql.process_node(object)
    if clean_tags(object.tags) then
        return
    end
    if is_poi(object.tags) then
        local row = {
            tags = object.tags,
            geom = object:as_point()
        }

        local associated_street_relation = n2as[object.id]
        if associated_street_relation then
            local refs = {}
            local ids = {}
            for rel_id, rel_name in pairs(associated_street_relation) do
                row.tags['addr:street'] = rel_name
            end
        end
        tables.node_poi:insert(row)
    end
end

function osm2pgsql.process_way(object)
    if clean_tags(object.tags) then
        return
    end
    if not object.is_closed and is_roadway(object.tags) then
        local row = {
            tags = object.tags,
            geom = object:as_linestring()
        }

        -- If there is any data from parent relations, add it in
        local d = w2r[object.id]
        if d then
            local refs = {}
            local ids = {}
            for rel_id, rel_ref in pairs(d) do
                ids[#ids + 1] = rel_id
            end
            table.sort(ids)
            row.rel_ids = '{' .. table.concat(ids, ',') .. '}'
        end
        tables.roads:insert(row)
    end
    if object.is_closed and is_poi(object.tags) then
        local row = {
            tags = object.tags,
            geom = object:as_polygon()
        }

        local associated_street_relation = n2as[object.id]
        if associated_street_relation then
            local refs = {}
            local ids = {}
            for rel_id, rel_name in pairs(associated_street_relation) do
                row.tags['addr:street'] = rel_name
            end
        end
        tables.way_poi:insert(row)
    end
end

-- This function is called for every added, modified, or deleted relation.
-- Its only job is to return the ids of all member nodes/ways of the specified
-- relation we want to see in stage 2 again. It MUST NOT store any information
-- about the relation!
function osm2pgsql.select_relation_members(relation)
    local node_ids = {}
    local way_ids = {}
    if is_restriction(relation.tags) then
        for _, member in ipairs(relation.members) do
            if member.type == 'n' then
                node_ids[#node_ids + 1] = member.ref
            elseif member.type == 'w' then
                way_ids[#way_ids + 1] = member.ref
            end
        end
    end
    if is_associated_street(relation.tags) then
        for _, member in ipairs(relation.members) do
            if member.type == 'n' then
                node_ids[#node_ids + 1] = member.ref
            elseif member.type == 'w' then
                way_ids[#way_ids + 1] = member.ref
            end
        end
    end

    return {
        nodes = node_ids,
        ways = way_ids,
    }
end

function osm2pgsql.process_relation(object)
    local relation_type = object:grab_tag('type')

    if is_restriction(object.tags) then
        local row = {
            tags = object.tags,
            members = object.members
        }
        for _, member in ipairs(object.members) do
            if member.type == 'n' then
                if not n2r[member.ref] then
                    n2r[member.ref] = {}
                end
                n2r[member.ref][object.id] = 'a'
            elseif member.type == 'w' then
                if not w2r[member.ref] then
                    w2r[member.ref] = {}
                end
                w2r[member.ref][object.id] = 'a'
            end
        end

        tables.restrictions:insert(row)
    end
    if is_associated_street(object.tags) then
        local row = {
            tags = object.tags,
            members = object.members
        }
        local name = object:grab_tag('name')
        if name then
            for _, member in ipairs(object.members) do
                if member.type == 'n' then
                    n2as[member.ref] = name
                elseif member.type == 'w' then
                    w2as[member.ref] = name
                end
            end
        end
    end
end

