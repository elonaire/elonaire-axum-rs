-- A schema-full user_id table
DEFINE TABLE user_id SCHEMAFULL;
-- define some fields
DEFINE FIELD user_id ON TABLE user_id TYPE string;
DEFINE INDEX userIndex ON TABLE user_id COLUMNS user_id UNIQUE;

-- A schema-full file_id table
DEFINE TABLE file_id SCHEMAFULL;
-- define some fields
DEFINE FIELD file_id ON TABLE file_id TYPE string;
DEFINE INDEX fileIndex ON TABLE file_id COLUMNS file_id UNIQUE;

-- A schema-full currency_id table
DEFINE TABLE currency_id SCHEMAFULL;
-- define some fields
DEFINE FIELD currency_id ON TABLE currency_id TYPE string;
DEFINE INDEX currencyIndex ON TABLE currency_id COLUMNS currency_id UNIQUE;

-- A schema-full professional_details table.
DEFINE TABLE OVERWRITE professional_details SCHEMAFULL TYPE NORMAL;
-- define some fields.
DEFINE FIELD occupation ON TABLE professional_details TYPE string;
DEFINE FIELD active ON TABLE professional_details TYPE bool
  DEFAULT false;
-- DEFINE EVENT enforce_single_true ON professional_details WHEN $event = 'CREATE' OR $event = 'UPDATE' THEN {
--     IF $after.active {
--         LET $existing_true = SELECT * FROM professional_details WHERE active = true;
--         IF array::len($existing_true) > 0 {
--             THROW 'You have an active professional detail already. Please deactivate it first.';
--         };
--     };
-- };
DEFINE INDEX occupationIndex ON TABLE professional_details COLUMNS occupation UNIQUE;
DEFINE FIELD description ON TABLE professional_details TYPE string;
-- I had to cast this way because the client submits a datetime string and explictly specifying datetime type gives an error
DEFINE FIELD start_date ON TABLE professional_details VALUE <datetime>$value;
DEFINE FIELD created_at ON TABLE professional_details TYPE datetime
  DEFAULT time::now() READONLY;
DEFINE FIELD updated_at ON TABLE professional_details TYPE datetime
  VALUE time::now();

-- A schema-full portfolio table.
DEFINE TABLE portfolio SCHEMAFULL TYPE NORMAL;
-- define some fields.
DEFINE FIELD title ON TABLE portfolio TYPE string;
DEFINE FIELD description ON TABLE portfolio TYPE string;
-- I had to cast this way because the client submits a datetime string and explictly specifying datetime type gives an error
DEFINE FIELD start_date ON TABLE portfolio VALUE <datetime>$value;
DEFINE FIELD end_date ON TABLE portfolio VALUE IF $value IS NOT NONE { <datetime>$value } ELSE { NONE };
DEFINE FIELD link ON TABLE portfolio TYPE string;
DEFINE FIELD thumbnail ON TABLE portfolio TYPE string
  ASSERT string::is::url($value);
DEFINE FIELD category ON TABLE portfolio TYPE string
-- Allow only these values in the array
  ASSERT $value INSIDE ["JavaScript", "Rust", "Database", "Cloud", "DevOps", "Mobile"];
DEFINE FIELD created_at ON TABLE portfolio TYPE datetime
  DEFAULT time::now() READONLY;
DEFINE FIELD updated_at ON TABLE portfolio TYPE datetime
  VALUE time::now();

-- A schema-full resume table.
DEFINE TABLE resume SCHEMAFULL TYPE NORMAL;
-- define some fields.
DEFINE FIELD section ON TABLE resume TYPE string
-- Allow only these values in the array
  ASSERT $value INSIDE ["Education", "Experience", "Achievements", "Projects", "Certifications", "Volunteer", "Publications", "Languages", "Interests", "References"];
DEFINE FIELD title ON TABLE resume TYPE string;
DEFINE FIELD more_info ON TABLE resume TYPE string;
-- I had to cast this way because the client submits a datetime string and explictly specifying datetime type gives an error
DEFINE FIELD start_date ON TABLE resume VALUE <datetime>$value;
DEFINE FIELD end_date ON TABLE resume VALUE IF $value IS NOT NONE { <datetime>$value } ELSE { NONE };
DEFINE FIELD link ON TABLE resume TYPE option<string>
  DEFAULT NONE;
DEFINE FIELD created_at ON TABLE resume TYPE datetime
  DEFAULT time::now() READONLY;
DEFINE FIELD updated_at ON TABLE resume TYPE datetime
  VALUE time::now();


-- A schema-full achievement table. Should relate to resume table
DEFINE TABLE achievement SCHEMAFULL TYPE RELATION IN resume OUT resume;
-- define some fields.
DEFINE FIELD description ON TABLE achievement TYPE string;
DEFINE EVENT enforce_bidirectionality ON TABLE achievement WHEN $event = 'CREATE' OR $event = 'UPDATE' THEN {
    IF $after.in != $after.out {
        THROW 'Forbidden.';
    };
};
DEFINE FIELD created_at ON TABLE achievement TYPE datetime
  DEFAULT time::now() READONLY;
DEFINE FIELD updated_at ON TABLE achievement TYPE datetime
  VALUE time::now();


-- Aschema-full service table.
DEFINE TABLE service SCHEMAFULL TYPE NORMAL;
-- define some fields.
DEFINE FIELD title ON TABLE service TYPE string;
DEFINE INDEX titleIndex ON TABLE service COLUMNS title UNIQUE;
DEFINE FIELD description ON TABLE service TYPE string;
DEFINE FIELD thumbnail ON TABLE service TYPE string
  ASSERT string::is::url($value);
--
DEFINE FIELD created_at ON TABLE service TYPE datetime
  DEFAULT time::now() READONLY;
DEFINE FIELD updated_at ON TABLE service TYPE datetime
  VALUE time::now();


-- A schema-full rate table.
DEFINE TABLE rate SCHEMAFULL TYPE NORMAL;
DEFINE FIELD OVERWRITE service ON TABLE rate TYPE record<service>;
DEFINE FIELD base_rate ON TABLE rate VALUE <float>$value;
DEFINE FIELD hour_week ON TABLE rate TYPE int DEFAULT 40;
DEFINE FIELD OVERWRITE currency_id ON TABLE rate TYPE record<currency_id>;
DEFINE INDEX serviceIndex ON TABLE rate COLUMNS service UNIQUE;
DEFINE FIELD created_at ON TABLE rate TYPE datetime
  DEFAULT time::now() READONLY;
DEFINE FIELD updated_at ON TABLE rate TYPE datetime
  VALUE time::now();


-- -- A schema-full discount table.
-- DEFINE TABLE discount SCHEMAFULL TYPE NORMAL;
-- DEFINE FIELD percentage ON TABLE discount TYPE int
--     ASSERT 0 > $value <= 1;
-- DEFINE FIELD start_date ON TABLE discount VALUE <datetime>$value;
-- DEFINE FIELD end_date ON TABLE discount VALUE <datetime>$value;
-- DEFINE FIELD created_at ON TABLE discount TYPE datetime
--   DEFAULT time::now() READONLY;
-- DEFINE FIELD updated_at ON TABLE discount TYPE datetime
--   VALUE time::now();

-- -- A schema-full applies_to table to relate discount to service.
-- DEFINE TABLE applies_to SCHEMAFULL TYPE RELATION IN discount OUT service;
-- DEFINE FIELD billing_interval ON TABLE applies_to TYPE string
--     ASSERT $value INSIDE ["Hourly", "Weekly", "Monthly", "Annual"];
-- DEFINE FIELD created_at ON TABLE applies_to TYPE datetime
--   DEFAULT time::now() READONLY;
-- DEFINE FIELD updated_at ON TABLE applies_to TYPE datetime
--   VALUE time::now();


-- This is not working at all, an issue with SurrealDB
-- A schema-full pre-computed billing_rates table view.
-- DEFINE TABLE OVERWRITE billing_rates SCHEMAFULL TYPE NORMAL AS
--     SELECT service.title AS service_title, service.id AS service_id, base_rate AS hourly_rate, base_rate * hour_week AS weekly_rate, base_rate * hour_week * 4 AS monthly_rate, base_rate * hour_week * 4 * 12 AS annual_rate FROM rate GROUP BY service_title;


-- A schema-full blog post table.
DEFINE TABLE OVERWRITE blog_post SCHEMAFULL TYPE NORMAL;
-- define some fields.
DEFINE FIELD title ON TABLE blog_post TYPE string;
DEFINE FIELD short_description ON TABLE blog_post TYPE string;
DEFINE FIELD thumbnail ON TABLE blog_post TYPE string
  ASSERT string::is::url($value);
DEFINE FIELD OVERWRITE content_file ON TABLE blog_post TYPE record<file_id>;
DEFINE FIELD content ON TABLE blog_post TYPE string;
DEFINE FIELD category ON TABLE blog_post TYPE string
-- Allow only these values in the array
  ASSERT $value INSIDE ["WebDevelopment", "MobileDevelopment", "ArtificialIntelligence", "Technology", "Lifestyle"];
DEFINE FIELD link ON TABLE blog_post VALUE string::slug(title);
DEFINE INDEX blogPostIndex ON TABLE blog_post COLUMNS link UNIQUE;
DEFINE FIELD status ON TABLE blog_post TYPE option<string>
-- Allow only these values in the array
  ASSERT $value INSIDE ["Draft", "Published", "Archived"]
  DEFAULT "Draft";
DEFINE FIELD published_date ON TABLE blog_post VALUE
  IF $value IS NOT NONE { <datetime>$value }ELSE { NONE };
DEFINE FIELD is_featured ON TABLE blog_post TYPE bool
  DEFAULT false;
DEFINE FIELD is_premium ON TABLE blog_post TYPE bool
  DEFAULT false;
DEFINE FIELD created_at ON TABLE blog_post TYPE datetime
  DEFAULT time::now() READONLY;
DEFINE FIELD updated_at ON TABLE blog_post TYPE datetime
  VALUE time::now();


-- A schema-full comment table.
DEFINE TABLE comment SCHEMAFULL TYPE NORMAL;
-- define some fields.
DEFINE FIELD content ON TABLE comment TYPE string;
DEFINE FIELD created_at ON TABLE comment TYPE datetime
  DEFAULT time::now() READONLY;
DEFINE FIELD updated_at ON TABLE comment TYPE datetime
  VALUE time::now();


-- A schema-full user_id-comment as writer relation table.
DEFINE TABLE wrote SCHEMAFULL TYPE RELATION IN user_id OUT comment | blog_post;

-- A schema-full blog_post-comment relation table.
DEFINE TABLE has_comment SCHEMAFULL TYPE RELATION IN blog_post OUT comment;

-- A schema-full comment-comment as reply relation table.
DEFINE TABLE has_reply SCHEMAFULL TYPE RELATION IN comment OUT comment;

-- A schema-full reaction table.
DEFINE TABLE OVERWRITE reaction SCHEMAFULL TYPE RELATION IN user_id OUT blog_post | comment;
-- define some fields.
DEFINE FIELD type ON TABLE reaction TYPE string
-- Allow only these values in the array
  ASSERT $value INSIDE ["Like", "Dislike", "Love", "Haha", "Wow", "Sad", "Angry"];
DEFINE INDEX reactionIndex ON TABLE reaction COLUMNS in,out UNIQUE;
DEFINE FIELD created_at ON TABLE reaction TYPE datetime
  DEFAULT time::now() READONLY;


-- A schema-full message table.
DEFINE TABLE message SCHEMAFULL;
-- define some fields.
DEFINE FIELD sender_name ON TABLE message TYPE string;
DEFINE FIELD sender_email ON TABLE message TYPE string
  ASSERT string::is::email($value);
DEFINE FIELD subject ON TABLE message TYPE string
-- Allow only these values in the array
  ASSERT $value INSIDE ["JobOffer", "Consultation", "Feedback", "Complaint", "Suggestion", "Enquiry"];
DEFINE FIELD body ON TABLE message TYPE string;
DEFINE FIELD created_at ON TABLE message TYPE datetime
  DEFAULT time::now() READONLY;


-- A schema-full notification table.
DEFINE TABLE notification SCHEMAFULL;
-- define some fields.
DEFINE FIELD content ON TABLE notification TYPE string;
DEFINE FIELD created_at ON TABLE notification TYPE datetime
  DEFAULT time::now() READONLY;

-- A schema-full skill table.
DEFINE TABLE skill SCHEMAFULL TYPE NORMAL;
-- define some fields.
DEFINE FIELD name ON TABLE skill TYPE string;
DEFINE FIELD description ON TABLE skill TYPE string;
DEFINE FIELD type ON TABLE skill TYPE string
-- Allow only these values in the array
  ASSERT $value INSIDE ["Technical", "Soft"];
DEFINE INDEX skillIndex ON TABLE skill COLUMNS name UNIQUE;
DEFINE FIELD thumbnail ON TABLE skill TYPE string
  ASSERT string::is::url($value);
DEFINE FIELD level ON TABLE skill TYPE option<string>
-- Allow only these values in the array
  ASSERT $value INSIDE ["Beginner", "Intermediate", "Advanced", "Expert", None]
  DEFAULT None;
-- I had to cast this way because the client submits a datetime string and explictly specifying datetime type gives an error
DEFINE FIELD start_date ON TABLE skill VALUE <datetime>$value;
DEFINE FIELD created_at ON TABLE skill TYPE datetime
  DEFAULT time::now() READONLY;
DEFINE FIELD updated_at ON TABLE skill TYPE datetime
  VALUE time::now();

-- Relationship between portfolio and skill
DEFINE TABLE uses_skill SCHEMAFULL TYPE RELATION IN portfolio OUT skill;
DEFINE INDEX skillIndex ON TABLE uses_skill
COLUMNS in, out UNIQUE;

-- Relationship between ratecard and service.
DEFINE TABLE ratecard SCHEMAFULL TYPE NORMAL;
-- define some fields.
DEFINE FIELD name ON TABLE ratecard TYPE string;
DEFINE FIELD created_at ON TABLE ratecard TYPE datetime
  DEFAULT time::now() READONLY;
DEFINE FIELD updated_at ON TABLE ratecard TYPE datetime
  VALUE time::now();


-- Relationship between service_request and service.
DEFINE TABLE service_request SCHEMAFULL TYPE NORMAL;
DEFINE FIELD description ON TABLE service_request TYPE string;
DEFINE FIELD OVERWRITE supporting_docs ON TABLE service_request TYPE array<record<file_id>>;
DEFINE FIELD start_date ON TABLE service_request VALUE <datetime>$value;
DEFINE FIELD end_date ON TABLE service_request VALUE <datetime>$value;
DEFINE FIELD created_at ON TABLE service_request TYPE datetime
  DEFAULT time::now() READONLY;
DEFINE FIELD updated_at ON TABLE service_request TYPE datetime
  VALUE time::now();

--
-- Relationship between service_request, ratecard and service.
DEFINE TABLE contains SCHEMAFULL TYPE RELATION IN service_request | ratecard OUT service;
DEFINE INDEX ratecardServiceRequestIndex ON TABLE contains
COLUMNS in, out UNIQUE;

/* Migrations */
-- Migration for blog post table
-- UPDATE blog_post SET is_featured = false, is_premium = false WHERE is_featured IS NONE OR is_premium IS NONE;
