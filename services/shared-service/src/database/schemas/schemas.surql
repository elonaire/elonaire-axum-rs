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

-- A schema-full professional_details table.
DEFINE TABLE professional_details SCHEMAFULL TYPE RELATION IN user_id OUT user_id;
-- define some fields.
DEFINE FIELD occupation ON TABLE professional_details TYPE string;
DEFINE FIELD active ON TABLE professional_details TYPE bool
  DEFAULT false;
DEFINE EVENT enforce_single_true ON professional_details WHEN $event = 'CREATE' OR $event = 'UPDATE' THEN {
    IF $before.active == true {
        LET $existing_true = SELECT * FROM professional_details WHERE active = true;
        IF array::len($existing_true) > 0 {
            THROW 'You have an active professional detail already. Please deactivate it first.';
        };
    };
};
DEFINE INDEX occupationIndex ON TABLE professional_details COLUMNS occupation UNIQUE;
DEFINE FIELD description ON TABLE professional_details TYPE string;
-- I had to cast this way because the client submits a datetime string and explictly specifying datetime type gives an error
DEFINE FIELD start_date ON TABLE professional_details VALUE <datetime>$value;
DEFINE FIELD created_at ON TABLE professional_details TYPE datetime
  DEFAULT time::now() READONLY;
DEFINE FIELD updated_at ON TABLE professional_details TYPE datetime
  VALUE time::now();

-- A schema-full portfolio table.
DEFINE TABLE portfolio SCHEMAFULL TYPE RELATION IN user_id OUT user_id;
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
DEFINE TABLE resume SCHEMAFULL TYPE RELATION IN user_id OUT user_id;
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
DEFINE FIELD created_at ON TABLE achievement TYPE datetime
  DEFAULT time::now() READONLY;
DEFINE FIELD updated_at ON TABLE achievement TYPE datetime
  VALUE time::now();


-- Aschema-full service table.
DEFINE TABLE service SCHEMAFULL TYPE RELATION IN user_id OUT user_id;
-- define some fields.
DEFINE FIELD title ON TABLE service TYPE string;
DEFINE INDEX titleIndex ON TABLE service COLUMNS title UNIQUE;
DEFINE FIELD description ON TABLE service TYPE string;
DEFINE FIELD thumbnail ON TABLE service TYPE string
  ASSERT string::is::url($value);
DEFINE FIELD created_at ON TABLE service TYPE datetime
  DEFAULT time::now() READONLY;
DEFINE FIELD updated_at ON TABLE service TYPE datetime
  VALUE time::now();


-- A schema-full blog post table.
DEFINE TABLE blog_post SCHEMAFULL TYPE RELATION IN user_id OUT user_id;
-- define some fields.
DEFINE FIELD title ON TABLE blog_post TYPE string;
DEFINE FIELD short_description ON TABLE blog_post TYPE string;
DEFINE FIELD thumbnail ON TABLE blog_post TYPE string
  ASSERT string::is::url($value);
DEFINE FIELD content_file ON TABLE blog_post TYPE record<file_id>;
DEFINE FIELD other_images ON TABLE blog_post TYPE array<string>;
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
DEFINE TABLE wrote SCHEMAFULL TYPE RELATION IN user_id OUT comment;

-- A schema-full blog_post-comment relation table.
DEFINE TABLE has_comment SCHEMAFULL TYPE RELATION IN blog_post OUT comment;

-- A schema-full comment-comment as reply relation table.
DEFINE TABLE has_reply SCHEMAFULL TYPE RELATION IN comment OUT comment;

-- A schema-full reaction table.
DEFINE TABLE reaction SCHEMAFULL TYPE RELATION IN user_id OUT blog_post | comment;
-- define some fields.
DEFINE FIELD type ON TABLE reaction TYPE string
-- Allow only these values in the array
  ASSERT $value INSIDE ["Like", "Dislike", "Love", "Haha", "Wow", "Sad", "Angry"];
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
DEFINE TABLE skill SCHEMAFULL TYPE RELATION IN user_id OUT user_id;
-- define some fields.
DEFINE FIELD name ON TABLE skill TYPE string;
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

/* Migrations */
-- Migration for blog post table
-- UPDATE blog_post SET is_featured = false, is_premium = false WHERE is_featured IS NONE OR is_premium IS NONE;
